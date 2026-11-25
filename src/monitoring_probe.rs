use crate::{
    model::Server,
    monitoring::{Config, Platform, Sample},
};
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    process::{Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

pub fn probe(server: &Server, config: &Config, cancel: &AtomicBool) -> Sample {
    probe_with_credentials(server, config, cancel, None)
}

pub fn probe_with_credentials(
    server: &Server,
    config: &Config,
    cancel: &AtomicBool,
    credential_root: Option<&std::path::Path>,
) -> Sample {
    let mut result = Sample {
        timestamp: crate::notifications::now(),
        ..Default::default()
    };
    if let Err(error) = server.validate() {
        result.error = Some(error);
        return result;
    }
    let start = Instant::now();
    let web = TcpStream::connect_timeout(
        &SocketAddr::new(server.ip, server.web_port),
        Duration::from_millis(800),
    );
    result.web = web.is_ok();
    if result.web {
        result.latency_ms = Some(start.elapsed().as_secs_f64() * 1000.0);
    }
    let start = Instant::now();
    result.ssh = TcpStream::connect_timeout(
        &SocketAddr::new(server.ip, server.ssh_port),
        Duration::from_millis(800),
    )
    .is_ok();
    if result.latency_ms.is_none() && result.ssh {
        result.latency_ms = Some(start.elapsed().as_secs_f64() * 1000.0);
    }
    if cancel.load(Ordering::Relaxed) {
        return result;
    }
    if let Ok(mut stream) = web {
        if server.https {
            match tls(server, cancel) {
                Ok((days, status)) => {
                    result.certificate_days = Some(days);
                    result.http_status = Some(status);
                }
                Err(e) => result.error = Some(e),
            }
        } else {
            let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
            let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
            let request = format!(
                "HEAD / HTTP/1.1\r\nHost: {}:{}\r\nConnection: close\r\n\r\n",
                server.ip, server.web_port
            );
            if stream.write_all(request.as_bytes()).is_ok() {
                let mut buffer = [0; 512];
                if let Ok(size) = stream.read(&mut buffer) {
                    result.http_status = http_status(&String::from_utf8_lossy(&buffer[..size]));
                }
            }
            if result.http_status.is_none() {
                result.error = Some("Web port open but no HTTP response".into());
            }
        }
    }
    if config.host_metrics && result.ssh && !cancel.load(Ordering::Relaxed) {
        match host_metrics(server, config, cancel, credential_root) {
            Ok(metrics) => {
                result.cpu = metrics.cpu;
                result.memory = metrics.memory;
                result.disk = metrics.disk;
                result.uptime = metrics.uptime;
                result.received = metrics.received;
                result.sent = metrics.sent;
                result.hardware = metrics.hardware;
            }
            Err(e) => {
                result.error = Some(match result.error {
                    Some(web) => format!("{web}; {e}"),
                    None => e,
                })
            }
        }
    }
    for check in config.services.iter().take(8) {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        result.services.push(crate::service_checks::probe(
            server,
            check,
            config,
            cancel,
            credential_root,
        ));
    }
    result
}
fn http_status(text: &str) -> Option<u16> {
    let line = text.lines().next()?;
    let mut words = line.split_whitespace();
    words.next()?.starts_with("HTTP/").then_some(())?;
    words
        .next()?
        .parse::<u16>()
        .ok()
        .filter(|n| (100..=599).contains(n))
}

// Fixed, read-only collectors. Remote shell parameters never contain user input.
const LINUX: &str = r#"LC_ALL=C; export LC_ALL
awk '/^cpu /{idle=$5+$6; total=0; for(i=2;i<=9;i++)total+=$i; print total,idle}' /proc/stat
sleep 1
awk '/^cpu /{idle=$5+$6; total=0; for(i=2;i<=9;i++)total+=$i; print total,idle}' /proc/stat
awk '/MemTotal:/{t=$2}/MemAvailable:/{a=$2}END{if(t>0)printf "memory=%.2f\n",100*(t-a)/t}' /proc/meminfo
df -P / | awk 'NR==2{gsub(/%/,"",$5);print "disk=" $5}'
awk '{print "uptime=" $1}' /proc/uptime
awk -F '[: ]+' 'NR>2 && $2!="lo"{r+=$3;s+=$11}END{print "received=" r;print "sent=" s}' /proc/net/dev
"#;
// UniFi OS consoles and BusyBox-based APs/switches expose Linux proc counters.
// Older firmware lacks MemAvailable: estimate reclaimable memory from free/cache.
const UNIFI: &str = r#"LC_ALL=C; export LC_ALL
awk '/^cpu /{idle=$5+$6; total=0; for(i=2;i<=9;i++)total+=$i; print total,idle}' /proc/stat
sleep 1
awk '/^cpu /{idle=$5+$6; total=0; for(i=2;i<=9;i++)total+=$i; print total,idle}' /proc/stat
awk '/^MemTotal:/{t=$2}/^MemAvailable:/{a=$2;found=1}/^MemFree:/{f=$2}/^Buffers:/{b=$2}/^Cached:/{c=$2}/^SReclaimable:/{r=$2}/^Shmem:/{s=$2}END{if(t>0){if(!found)a=f+b+c+r-s;if(a<0)a=0;if(a>t)a=t;printf "memory=%.2f\n",100*(t-a)/t}}' /proc/meminfo
df -P / 2>/dev/null | awk 'NR==2{for(i=1;i<=NF;i++)if($i ~ /^[0-9]+%$/){gsub(/%/,"",$i);print "disk=" $i;break}}'
awk '{print "uptime=" $1}' /proc/uptime
awk -F '[: ]+' 'NR>2 && $2!="lo"{r+=$3;s+=$11}END{print "received=" r;print "sent=" s}' /proc/net/dev
"#;
const MAC: &str = r#"LC_ALL=C; export LC_ALL
top -l 2 -n 0 -s 1 | awk '/CPU usage:/{v=$7}END{gsub(/%/,"",v);print "cpu=" 100-v}'
vm_stat | awk '/page size of/{p=$8}/Pages free:/{f=$3}/Pages inactive:/{i=$3}/Pages speculative:/{s=$3}END{gsub(/\./,"",f);gsub(/\./,"",i);gsub(/\./,"",s);printf "available=%.0f\n",(f+i+s)*p}'
sysctl -n hw.memsize | awk '{print "total=" $1}'
df -P / | awk 'NR==2{gsub(/%/,"",$5);print "disk=" $5}'
sysctl -n kern.boottime | awk -v n="$(date +%s)" '{gsub(/,/,"",$4);print "uptime=" n-$4}'
netstat -ibn | awk '$3 ~ /^<Link/ && $1 !~ /^lo/{r+=$7;s+=$10}END{print "received=" r;print "sent=" s}'
"#;
const WINDOWS: &str = r#"$ErrorActionPreference='Stop';$c=[Globalization.CultureInfo]::InvariantCulture;[Threading.Thread]::CurrentThread.CurrentCulture=$c;$o=Get-CimInstance Win32_OperatingSystem;$p=Get-CimInstance Win32_Processor;$d=Get-CimInstance Win32_LogicalDisk -Filter ('DeviceID='''+$env:SystemDrive+'''');'cpu='+[string](($p|Measure-Object LoadPercentage -Average).Average);'memory='+[string](100*(1-$o.FreePhysicalMemory/$o.TotalVisibleMemorySize));'disk='+[string](100*(1-$d.FreeSpace/$d.Size));'uptime='+[string]([datetime]::Now-$o.LastBootUpTime).TotalSeconds;$n=Get-NetAdapterStatistics;'received='+[string](($n|Measure-Object ReceivedBytes -Sum).Sum);'sent='+[string](($n|Measure-Object SentBytes -Sum).Sum)"#;
fn encoded(text: &str) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes: Vec<_> = text.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let mut out = String::new();
    for c in bytes.chunks(3) {
        let n = ((c[0] as u32) << 16)
            | ((c.get(1).copied().unwrap_or(0) as u32) << 8)
            | c.get(2).copied().unwrap_or(0) as u32;
        out.push(TABLE[(n >> 18) as usize] as char);
        out.push(TABLE[((n >> 12) & 63) as usize] as char);
        out.push(if c.len() > 1 {
            TABLE[((n >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if c.len() > 2 {
            TABLE[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}
fn host_metrics(
    s: &Server,
    config: &Config,
    cancel: &AtomicBool,
    credential_root: Option<&std::path::Path>,
) -> Result<Sample, String> {
    let platform = config.platform;
    if platform == Platform::Idrac {
        return crate::idrac::collect(s, config, cancel, credential_root).map(|hardware| Sample {
            hardware: Some(hardware),
            ..Default::default()
        });
    }
    let remote = match platform {
        Platform::Idrac => unreachable!(),
        Platform::Linux => LINUX.into(),
        Platform::Unifi => UNIFI.into(),
        Platform::MacOs => MAC.into(),
        Platform::Windows => format!(
            "powershell.exe -NoLogo -NoProfile -NonInteractive -EncodedCommand {}",
            encoded(WINDOWS)
        ),
    };
    parse_metrics(
        &ssh_output(s, config, cancel, credential_root, &remote)?,
        platform,
    )
}
pub(crate) fn ssh_output(
    s: &Server,
    config: &Config,
    cancel: &AtomicBool,
    credential_root: Option<&std::path::Path>,
    remote: &str,
) -> Result<String, String> {
    let mut command = Command::new("ssh");
    if let Some(option) = crate::ssh_trust::known_hosts_option(s, credential_root)? {
        command.args(["-o", &option]);
    }
    command.args(["-o", "GSSAPIAuthentication=no"]);
    let password_auth = config.uses_password();
    if !password_auth && let Some(path) = config.ssh_identity()? {
        command
            .arg("-i")
            .arg(path)
            .args(["-o", "IdentitiesOnly=yes"]);
    }
    if password_auth {
        let id = config.password_id.as_deref().ok_or(
            "Save an SSH password in Monitoring settings before using password authentication",
        )?;
        crate::monitoring_credentials::configure(
            &mut command,
            credential_root.ok_or("SSH credential storage is unavailable")?,
            id,
            s,
        )?;
        command.args([
            "-o",
            "PreferredAuthentications=password,keyboard-interactive",
            "-o",
            "PubkeyAuthentication=no",
            "-o",
            "NumberOfPasswordPrompts=1",
        ]);
    }
    command.args([
        "-T",
        "-o",
        if password_auth {
            "BatchMode=no"
        } else {
            "BatchMode=yes"
        },
        "-o",
        "StrictHostKeyChecking=yes",
        "-o",
        "ConnectTimeout=5",
        "-o",
        "ConnectionAttempts=1",
        "-o",
        "ServerAliveInterval=3",
        "-o",
        "ServerAliveCountMax=1",
        "-p",
        &s.ssh_port.to_string(),
        "-l",
        &s.username,
        &s.ip.to_string(),
        remote,
    ]);
    let output=bounded_output(command,cancel).map_err(|e|format!("SSH metrics unavailable: {e}. Verify the SSH username and host key. Save a monitoring password or configure a key/agent login."))?;
    Ok(output)
}
pub fn parse_metrics(output: &str, platform: Platform) -> Result<Sample, String> {
    let mut sample = Sample::default();
    let mut counters = Vec::new();
    let mut available = None;
    let mut total = None;
    for line in output.lines() {
        if let Some((key, value)) = line.split_once('=') {
            let Ok(value) = value.trim().parse::<f64>() else {
                continue;
            };
            if !value.is_finite() || value < 0.0 {
                continue;
            }
            match key.trim() {
                "cpu" if value <= 100.0 => sample.cpu = Some(value),
                "memory" if value <= 100.0 => sample.memory = Some(value),
                "disk" if value <= 100.0 => sample.disk = Some(value),
                "uptime" => sample.uptime = Some(value),
                "received" => sample.received = Some(value),
                "sent" => sample.sent = Some(value),
                "available" => available = Some(value),
                "total" => total = Some(value),
                _ => {}
            }
        } else if matches!(platform, Platform::Linux | Platform::Unifi) {
            let values: Vec<f64> = line
                .split_whitespace()
                .filter_map(|v| v.parse().ok())
                .collect();
            if values.len() == 2 && values.iter().all(|v| v.is_finite() && *v >= 0.0) {
                counters.push(values);
            }
        }
    }
    if matches!(platform, Platform::Linux | Platform::Unifi) && counters.len() == 2 {
        let dt = counters[1][0] - counters[0][0];
        let idle = counters[1][1] - counters[0][1];
        if dt > 0.0 && idle >= 0.0 && idle <= dt {
            sample.cpu = Some(100.0 * (dt - idle) / dt);
        }
    }
    if let (Some(a), Some(t)) = (available, total)
        && t > 0.0
        && a <= t
    {
        sample.memory = Some(100.0 * (t - a) / t);
    }
    if sample.cpu.is_none() && sample.memory.is_none() && sample.disk.is_none() {
        return Err("SSH connected but the selected platform did not return host metrics".into());
    }
    Ok(sample)
}

fn bounded_output(mut command: Command, cancel: &AtomicBool) -> Result<String, String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|e| e.to_string())?;
    let stdout = child.stdout.take().ok_or("No process output")?;
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stdout.take(65536).read_to_end(&mut bytes);
        bytes
    });
    let stderr = child.stderr.take().ok_or("No error output")?;
    let errors = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stderr.take(8192).read_to_end(&mut bytes);
        bytes
    });
    let start = Instant::now();
    let status = loop {
        if cancel.load(Ordering::Relaxed) || start.elapsed() > Duration::from_secs(12) {
            let _ = child.kill();
            let _ = child.wait();
            let _ = reader.join();
            let _ = errors.join();
            return Err("request timed out or was cancelled".into());
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => std::thread::sleep(Duration::from_millis(40)),
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = reader.join();
                let _ = errors.join();
                return Err(e.to_string());
            }
        }
    };
    let bytes = reader.join().map_err(|_| "Cannot read process output")?;
    let error_bytes = errors.join().map_err(|_| "Cannot read process errors")?;
    if !status.success() {
        return Err(process_error(&error_bytes, status.code().unwrap_or(-1)));
    }
    String::from_utf8(bytes).map_err(|_| "Invalid response encoding".into())
}
fn process_error(bytes: &[u8], code: i32) -> String {
    let text = String::from_utf8_lossy(bytes);
    let lower = text.to_lowercase();
    if lower.contains("permission denied") {
        return "Authentication rejected: the SSH username/password or key was not accepted".into();
    }
    if lower.contains("host key verification failed")
        || lower.contains("host identification has changed")
    {
        return "Host key verification failed. Verify this device through SSH Shell before monitoring".into();
    }
    let detail: String = text
        .lines()
        .find(|s| !s.trim().is_empty())
        .unwrap_or("")
        .chars()
        .filter(|c| !c.is_control())
        .take(240)
        .collect();
    if detail.is_empty() {
        format!("process exited with {code}")
    } else {
        format!("{detail} (exit {code})")
    }
}
fn tls(s: &Server, cancel: &AtomicBool) -> Result<(i64, u16), String> {
    // SslStream validates the certificate and hostname; validation is never bypassed.
    #[cfg(windows)]
    {
        let script = format!(
            r#"$ErrorActionPreference='Stop';$t=New-Object Net.Sockets.TcpClient;$t.Connect('{}',{});$t.ReceiveTimeout=3000;$t.SendTimeout=3000;$ssl=New-Object Net.Security.SslStream($t.GetStream(),$false);try{{$ssl.AuthenticateAsClient('{}');$cert=New-Object Security.Cryptography.X509Certificates.X509Certificate2($ssl.RemoteCertificate);$days=[math]::Floor(($cert.NotAfter.ToUniversalTime()-[datetime]::UtcNow).TotalDays);$request=[Text.Encoding]::ASCII.GetBytes("HEAD / HTTP/1.1`r`nHost: {}:{}`r`nConnection: close`r`n`r`n");$ssl.Write($request,0,$request.Length);$buffer=New-Object byte[] 512;$n=$ssl.Read($buffer,0,512);Write-Output $days;Write-Output ([Text.Encoding]::ASCII.GetString($buffer,0,$n))}}finally{{$ssl.Dispose();$t.Dispose()}}"#,
            s.ip, s.web_port, s.ip, s.ip, s.web_port
        );
        let mut command = Command::new("powershell.exe");
        command.args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-EncodedCommand",
            &encoded(&script),
        ]);
        let output = bounded_output(command, cancel)
            .map_err(|e| format!("HTTPS validation or request failed: {e}"))?;
        let (days, response) = output.split_once('\n').ok_or("Invalid HTTPS response")?;
        Ok((
            days.trim()
                .parse()
                .map_err(|_| "Invalid certificate expiry")?,
            http_status(response).ok_or("No HTTPS response")?,
        ))
    }
    #[cfg(not(windows))]
    {
        if cancel.load(Ordering::Relaxed) {
            return Err("Check cancelled".into());
        }
        let stream =
            TcpStream::connect_timeout(&SocketAddr::new(s.ip, s.web_port), Duration::from_secs(3))
                .map_err(|e| e.to_string())?;
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .map_err(|e| e.to_string())?;
        stream
            .set_write_timeout(Some(Duration::from_secs(3)))
            .map_err(|e| e.to_string())?;
        let connector = native_tls::TlsConnector::new().map_err(|e| e.to_string())?;
        let mut tls = connector
            .connect(&s.ip.to_string(), stream)
            .map_err(|e| format!("HTTPS certificate validation failed: {e}"))?;
        let certificate = tls
            .peer_certificate()
            .map_err(|e| e.to_string())?
            .ok_or("Server did not send a certificate")?
            .to_der()
            .map_err(|e| e.to_string())?;
        let (_, certificate) = x509_parser::parse_x509_certificate(&certificate)
            .map_err(|_| "Invalid TLS certificate")?;
        let days = (certificate.validity().not_after.timestamp()
            - crate::notifications::now() as i64)
            .div_euclid(86400);
        write!(
            tls,
            "HEAD / HTTP/1.1\r\nHost: {}:{}\r\nConnection: close\r\n\r\n",
            s.ip, s.web_port
        )
        .map_err(|e| e.to_string())?;
        let mut response = [0u8; 512];
        let count = tls.read(&mut response).map_err(|e| e.to_string())?;
        Ok((
            days,
            http_status(&String::from_utf8_lossy(&response[..count])).ok_or("No HTTPS response")?,
        ))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unifi_collector_parses_busybox_counters_and_missing_metrics() {
        let s=parse_metrics("2400 1200\n2600 1300\nmemory=42.50\ndisk=18\nuptime=9000\nreceived=100000\nsent=200000\n",Platform::Unifi).unwrap();
        assert_eq!(s.cpu, Some(50.0));
        assert_eq!(s.memory, Some(42.5));
        assert_eq!(s.disk, Some(18.0));
        let partial = parse_metrics("1000 700\n1100 770\nuptime=800", Platform::Unifi).unwrap();
        assert!(partial.memory.is_none());
        assert!(partial.disk.is_none());
        assert_eq!(
            serde_json::from_str::<Platform>("\"Unifi\"").unwrap(),
            Platform::Unifi
        );
    }
    #[test]
    fn ssh_errors_explain_authentication_and_host_key_failures() {
        assert!(
            process_error(
                b"eebell@192.0.2.1: Permission denied (publickey,password).",
                255
            )
            .contains("Authentication rejected")
        );
        assert!(
            process_error(b"Host key verification failed.", 255)
                .contains("Host key verification failed")
        );
        assert_eq!(process_error(b"", 255), "process exited with 255");
    }
    #[test]
    fn linux_counter_deltas_and_metric_validation() {
        let sample = parse_metrics(
            "1000 700\n1200 800\nmemory=60\ndisk=80\nuptime=400\ncpu=NaN\n",
            Platform::Linux,
        )
        .unwrap();
        assert_eq!(sample.cpu, Some(50.0));
        assert_eq!(sample.memory, Some(60.0));
        assert!(parse_metrics("cpu=NaN\nmemory=200\ndisk=-1", Platform::Windows).is_err());
    }
    #[test]
    fn windows_and_mac_collectors_parse_without_faking_missing_values() {
        assert_eq!(
            parse_metrics("cpu=12.5\nmemory=44\ndisk=70", Platform::Windows)
                .unwrap()
                .cpu,
            Some(12.5)
        );
        assert_eq!(
            parse_metrics("available=25\ntotal=100\ndisk=5", Platform::MacOs)
                .unwrap()
                .memory,
            Some(75.0)
        );
        assert_eq!(http_status("HTTP/1.1 503 Unavailable\r\n"), Some(503));
        assert_eq!(http_status("SSH-2.0-OpenSSH"), None);
    }
    #[test]
    fn probe_reads_real_local_http_service() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let worker = std::thread::spawn(move || {
            for _ in 0..2 {
                let (mut stream, _) = listener.accept().unwrap();
                let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
                let mut buf = [0; 512];
                if stream.read(&mut buf).unwrap_or(0) > 0 {
                    let _ =
                        stream.write_all(b"HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n");
                }
            }
        });
        let mut server = Server::new(addr.ip());
        server.web_port = addr.port();
        server.ssh_port = addr.port();
        let sample = probe(&server, &Config::default(), &AtomicBool::new(false));
        worker.join().unwrap();
        assert!(sample.online());
        assert_eq!(sample.http_status, Some(204));
        assert!(sample.cpu.is_none());
    }
}
