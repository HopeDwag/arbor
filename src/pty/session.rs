use anyhow::{Context, Result};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

pub struct PtySession {
    writer: Box<dyn Write + Send>,
    parser: Arc<Mutex<vt100_ctt::Parser>>,
    _child: Box<dyn portable_pty::Child + Send + Sync>,
    master: Box<dyn portable_pty::MasterPty + Send>,
}

impl PtySession {
    pub fn spawn(cmd: &str, args: &[String], rows: u16, cols: u16, cwd: &Path) -> Result<Self> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("Failed to open PTY")?;

        let mut command = CommandBuilder::new(cmd);
        for arg in args {
            command.arg(arg);
        }
        command.cwd(cwd);

        let child = pair
            .slave
            .spawn_command(command)
            .context("Failed to spawn command in PTY")?;

        let writer = pair
            .master
            .take_writer()
            .context("Failed to get PTY writer")?;
        let mut reader = pair
            .master
            .try_clone_reader()
            .context("Failed to get PTY reader")?;

        let parser = Arc::new(Mutex::new(vt100_ctt::Parser::new(rows, cols, 1000)));

        // Spawn reader thread — feeds PTY output into vt100 parser
        let parser_clone = Arc::clone(&parser);
        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let mut p = parser_clone.lock().unwrap();
                        p.process(&buf[..n]);
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(Self {
            writer,
            parser,
            _child: child,
            master: pair.master,
        })
    }

    pub fn write(&mut self, data: &[u8]) -> Result<()> {
        self.writer.write_all(data)?;
        self.writer.flush()?;
        Ok(())
    }

    pub fn resize(&self, rows: u16, cols: u16) -> Result<()> {
        self.master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("Failed to resize PTY")?;
        let mut parser = self.parser.lock().unwrap();
        parser.screen_mut().set_size(rows, cols);
        Ok(())
    }

    pub fn screen(&self) -> Arc<Mutex<vt100_ctt::Parser>> {
        Arc::clone(&self.parser)
    }
}
