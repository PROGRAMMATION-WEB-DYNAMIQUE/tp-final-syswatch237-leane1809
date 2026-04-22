use chrono::Local;
use std::fmt;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use sysinfo::System;

#[derive(Debug, Clone)]
struct CpuInfo { usage: f32 }
#[derive(Debug, Clone)]
struct MemInfo { total: u64, used: u64 }
#[derive(Debug, Clone)]
struct ProcessInfo { pid: String, name: String, cpu: f32 }
#[derive(Debug, Clone)]
struct SystemSnapshot { cpu: CpuInfo, mem: MemInfo, processes: Vec<ProcessInfo>, time: String }

impl fmt::Display for CpuInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CPU Usage: {:.2}%", self.usage)
    }
}
impl fmt::Display for MemInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RAM Used: {} / {} MB", self.used/1024/1024, self.total/1024/1024)
    }
}
impl fmt::Display for ProcessInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:<8} {:<25} {:>6.2}%", self.pid, self.name, self.cpu)
    }
}
impl fmt::Display for SystemSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Snapshot @ {}", self.time)?;
        writeln!(f, "{}", self.cpu)?;
        writeln!(f, "{}", self.mem)?;
        writeln!(f, "Top Processes:")?;
        for p in &self.processes { writeln!(f, "{}", p)?; }
        Ok(())
    }
}

fn collect_snapshot() -> Result<SystemSnapshot, String> {
    let mut sys = System::new_all();
    sys.refresh_all();
    let cpu = CpuInfo { usage: sys.global_cpu_info().cpu_usage() };
    let mem = MemInfo { total: sys.total_memory(), used: sys.used_memory() };
    let mut processes: Vec<ProcessInfo> = sys.processes()
        .iter()
        .map(|(pid, p)| ProcessInfo {
            pid: pid.to_string(),
            name: p.name().to_string(),
            cpu: p.cpu_usage(),
        }).collect();
    processes.sort_by(|a,b| b.cpu.partial_cmp(&a.cpu).unwrap());
    processes.truncate(5);
    Ok(SystemSnapshot {
        cpu, mem, processes,
        time: Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
    })
}

fn bar(p:f32)->String{
    let n=(p/5.0).round() as usize;
    format!("[{:<20}] {:.2}%", "#".repeat(n.min(20)), p)
}

fn format_response(s:&SystemSnapshot, command:&str)->String{
    match command.trim().to_lowercase().as_str() {
        "cpu" => format!("{}\n{}\n", s.cpu, bar(s.cpu.usage)),
        "mem" => format!("{}\n", s.mem),
        "ps" => {
            let mut out=String::from("Top Processes:\n");
            for p in &s.processes { out.push_str(&format!("{}\n", p));}
            out
        },
        "all" => format!("{}\n", s),
        "help" => "Commands: cpu | mem | ps | all | help | quit\n".to_string(),
        "quit" => "Bye!\n".to_string(),
        _ => "Unknown command. Type help\n".to_string()
    }
}

fn log_event(msg:&str){
    if let Ok(mut f)=OpenOptions::new().create(true).append(true).open("syswatch.log"){
        let _=writeln!(f, "[{}] {}", Local::now().format("%Y-%m-%d %H:%M:%S"), msg);
    }
}

fn handle_client(mut stream:TcpStream, shared:Arc<Mutex<SystemSnapshot>>) {
    let addr = stream.peer_addr().map(|a| a.to_string()).unwrap_or("?".into());
    log_event(&format!("CONNECT {}", addr));
    let _=stream.write_all(b"Welcome to SysWatch. Type help\n");
    let reader_stream=stream.try_clone().unwrap();
    let mut reader=BufReader::new(reader_stream);
    loop {
        let _=stream.write_all(b"> ");
        let mut line=String::new();
        match reader.read_line(&mut line) {
            Ok(0)|Err(_) => break,
            Ok(_) => {
                let cmd=line.trim();
                log_event(&format!("{} CMD {}", addr, cmd));
                let snap=shared.lock().unwrap().clone();
                let resp=format_response(&snap, cmd);
                let _=stream.write_all(resp.as_bytes());
                if cmd.eq_ignore_ascii_case("quit"){break;}
            }
        }
    }
    log_event(&format!("DISCONNECT {}", addr));
}

fn main() {
    let first = collect_snapshot().unwrap();
    let shared = Arc::new(Mutex::new(first));
    let bg = Arc::clone(&shared);
    thread::spawn(move || loop {
        if let Ok(s)=collect_snapshot() {
            if let Ok(mut data)=bg.lock(){ *data=s; }
        }
        thread::sleep(Duration::from_secs(5));
    });

    let listener=TcpListener::bind("0.0.0.0:7878").expect("bind failed");
    println!("SysWatch listening on port 7878");
    for stream in listener.incoming() {
        if let Ok(stream)=stream {
            let data=Arc::clone(&shared);
            thread::spawn(move || handle_client(stream, data));
        }
    }
}
