//! Finite, owned inputs. Descendant mode requires a dedicated test subreaper.
#[cfg(target_os = "linux")]
use std::io::{self, Read, Write};
use std::process::ExitCode;
#[cfg(target_os = "linux")]
use std::time::Duration;

#[cfg(target_os = "linux")]
fn run() -> io::Result<u8> {
    use std::os::unix::ffi::OsStrExt;

    let args: Vec<_> = std::env::args_os().collect();
    let mut stdout = io::stdout().lock();
    let mut stderr = io::stderr().lock();
    match args.get(1).and_then(|value| value.to_str()) {
        Some("snapshot-source") => {
            let mut bytes = Vec::new();
            std::fs::File::open("source.txt")?
                .take(4097)
                .read_to_end(&mut bytes)?;
            if bytes.len() > 4096 {
                return Ok(65);
            }
            stdout.write_all(&bytes)?;
        }
        Some("context") => {
            // NUL-delimited fields preserve empty arguments and embedded newlines.
            for arg in &args {
                stdout.write_all(arg.as_bytes())?;
                stdout.write_all(b"\0")?;
            }
            stdout.write_all(std::env::current_dir()?.as_os_str().as_bytes())?;
            stdout.write_all(b"\0")?;
            // Inspect only this child environment. Never print unknown values.
            write!(stdout, "{}\0", std::env::vars_os().count())?;
            for key in ["GRAPH_FIXTURE_VALUE", "GRAPH_FIXTURE_EMPTY"] {
                let value = std::env::var_os(key).ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidInput, "missing fixture variable")
                })?;
                stdout.write_all(value.as_bytes())?;
                stdout.write_all(b"\0")?;
            }
            let mut byte = [0_u8; 1];
            write!(stdout, "stdin={}\0", io::stdin().read(&mut byte)?)?;
        }
        Some("binary") => {
            stdout.write_all(b"\0\xffout\r\nno-final-newline")?;
            stderr.write_all(b"\xfe\0err\n\r\x80")?;
            stdout.flush()?;
            stderr.flush()?;
            return Ok(23);
        }
        Some(mode @ ("flood-stdout" | "flood-stderr")) => {
            let output: &mut dyn Write = if mode == "flood-stdout" {
                &mut stdout
            } else {
                &mut stderr
            };
            // Exactly 1 MiB at most, even if the runner fails to enforce its cap.
            for _ in 0..128 {
                output.write_all(&[0xa5; 8192])?;
            }
        }
        Some("flood-both") => {
            drop(stdout);
            drop(stderr);
            // Independent writers exercise pressure on both pipes concurrently.
            let out = std::thread::spawn(|| -> io::Result<()> {
                let mut stream = io::stdout().lock();
                for _ in 0..64 {
                    stream.write_all(&[0xa5; 8192])?;
                }
                stream.flush()
            });
            let err = std::thread::spawn(|| -> io::Result<()> {
                let mut stream = io::stderr().lock();
                for _ in 0..64 {
                    stream.write_all(&[0x5a; 8192])?;
                }
                stream.flush()
            });
            // Join both even if either write fails; never detach a fixture writer.
            let out_result = out.join();
            let err_result = err.join();
            out_result.map_err(|_| io::Error::other("stdout fixture thread"))??;
            err_result.map_err(|_| io::Error::other("stderr fixture thread"))??;
            return Ok(0);
        }
        Some(mode @ ("rpc-exact" | "rpc-oversized" | "rpc-malformed" | "rpc-truncated")) => {
            use io::BufRead;
            let mut request = Vec::new();
            io::BufReader::new(io::stdin().lock())
                .take(4097)
                .read_until(b'\n', &mut request)?;
            if request.len() > 4096 || request.last() != Some(&b'\n') {
                return Ok(65);
            }
            let bytes = match mode {
                "rpc-exact" => {
                    let mut bytes = br#"{"id":1,"result":{"userAgent":"fixture","platformFamily":"unix","platformOs":"linux"}}"#.to_vec();
                    bytes.resize(4095, b' ');
                    bytes.push(b'\n');
                    bytes
                }
                "rpc-oversized" => {
                    let mut bytes = vec![b'x'; 4096];
                    bytes.push(b'\n');
                    bytes
                }
                "rpc-malformed" => b"{\n".to_vec(),
                _ => b"{\"id\":1".to_vec(),
            };
            stdout.write_all(&bytes)?;
        }
        Some(mode @ ("rpc-peer" | "rpc-stderr" | "rpc-no-read" | "rpc-context")) => {
            use io::BufRead;
            if mode == "rpc-context" {
                let mut starts = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("rpc-starts")?;
                writeln!(starts, "{}", std::process::id())?;
            }
            let mut input = io::BufReader::new(io::stdin().lock());
            for phase in 0..3 {
                let mut bytes = Vec::new();
                (&mut input).take(4097).read_until(b'\n', &mut bytes)?;
                if bytes.len() > 4096 || bytes.last() != Some(&b'\n') {
                    return Ok(65);
                }
                let request: serde_json::Value =
                    serde_json::from_slice(&bytes).map_err(io::Error::other)?;
                if mode == "rpc-context" && phase == 0 {
                    let context = serde_json::json!({
                        "argv": std::env::args().skip(1).collect::<Vec<_>>(),
                        "cwd": std::env::current_dir()?.to_string_lossy(),
                        "environment": std::env::vars().collect::<std::collections::BTreeMap<_, _>>(),
                        "initialize": request,
                    });
                    std::fs::write(
                        "rpc-context.json",
                        serde_json::to_vec(&context).map_err(io::Error::other)?,
                    )?;
                }
                let expected = ["initialize", "initialized", "fixture/ping"][phase];
                if request["method"] != expected {
                    return Ok(66);
                }
                if phase == 1 {
                    if mode == "rpc-no-read" {
                        std::fs::write("input-paused", b"initialized consumed")?;
                        std::thread::sleep(std::time::Duration::from_secs(2));
                        return Ok(0);
                    }
                    continue;
                }
                let result = if phase == 0 {
                    serde_json::json!({"userAgent":"owned-fixture", "platformFamily":"unix", "platformOs":"linux"})
                } else {
                    serde_json::json!({"pong":true})
                };
                let reply = serde_json::json!({"id":request["id"], "result":result});
                let mut bytes = serde_json::to_vec(&reply).map_err(io::Error::other)?;
                bytes.push(b'\n');
                stdout.write_all(&bytes)?;
                stdout.flush()?;
                if mode == "rpc-stderr" {
                    for _ in 0..32 {
                        stderr.write_all(&[0xa5; 8192])?;
                    }
                    stderr.flush()?;
                }
            }
        }
        Some("read-input") => {
            let mut bytes = Vec::new();
            io::stdin().take(1024 * 1024 + 1).read_to_end(&mut bytes)?;
            if bytes.len() > 1024 * 1024 {
                return Ok(65);
            }
            std::fs::write("received-input", bytes)?;
        }
        Some("visibility") => {
            for path in args.iter().skip(2) {
                let state = if std::path::Path::new(path).exists() {
                    "visible"
                } else {
                    "hidden"
                };
                writeln!(stdout, "{}={state}", path.to_string_lossy())?;
            }
        }
        Some("spawn-holder") => {
            if std::env::var("GRAPH_FIXTURE_SUBREAPER").as_deref() != Ok("1") {
                return Ok(64);
            }
            let mut holder = std::process::Command::new(std::env::current_exe()?)
                .arg("hold-pipes")
                .env_clear()
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
                .spawn()?;
            // Publish identity before deliberately orphaning to the test subreaper.
            if let Err(error) = std::fs::write("holder-pid", holder.id().to_string()) {
                let _ = holder.kill();
                let _ = holder.wait();
                return Err(error);
            }
            drop(holder);
            return Ok(0);
        }
        Some("hold-pipes") => {
            std::thread::sleep(Duration::from_millis(1000));
        }
        Some("sleep") => {
            // The cancellation test observes readiness before setting its flag.
            std::fs::write("ready", b"ready")?;
            std::thread::sleep(Duration::from_millis(1000));
        }
        _ => return Ok(64),
    }
    stdout.flush()?;
    stderr.flush()?;
    Ok(0)
}

#[cfg(target_os = "linux")]
fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(_) => ExitCode::from(74),
    }
}

#[cfg(not(target_os = "linux"))]
fn main() -> ExitCode {
    ExitCode::from(64)
}
