use std::process::ExitStatus;

use async_trait::async_trait;

use tokio::io::AsyncReadExt;

use super::output::CommandOutput;

#[async_trait]
pub trait ChildExt {
    async fn wait_and_pipe_output(&mut self, output: &mut CommandOutput) -> Result<ExitStatus, ()>;
}

#[async_trait]
impl ChildExt for tokio::process::Child {
    async fn wait_and_pipe_output(&mut self, output: &mut CommandOutput) -> Result<ExitStatus, ()> {
        let mut child_stdout = self.stdout.take().expect("to get the stdout of a child");
        let mut child_stderr = self.stderr.take().expect("to get the stdout of a child");

        let mut stdout_buf = [0u8; 1024];
        let mut stderr_buf = [0u8; 1024];

        loop {
            tokio::select! {
                stdout_read = child_stdout.read(&mut stdout_buf) => {
                    match stdout_read {
                        Ok(len) => {
                            output.stdout(&stdout_buf[0..len])
                        }
                        Err(e) => {
                            // TODO: return actual errors...
                            return Err(());
                        }
                    }
                },

                stderr_read = child_stderr.read(&mut stderr_buf) => {
                    match stderr_read {
                        Ok(len) => {
                            output.stderr(&stderr_buf[0..len]);
                        }
                        Err(_) => {
                            return Err(());
                        }
                    }
                }

                result = self.wait() => {
                    // Done so... do something?
                    // For now just return tbh.
                    // At the very least I need to check return codes.
                    return Ok(result.expect("waiting on child to succeed. TODO: fix me"));
                }
            }
        }
    }
}
