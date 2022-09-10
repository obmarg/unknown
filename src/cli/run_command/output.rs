// TODO: make this a submodule of run_command.
use std::{collections::HashMap, io::Write, iter::Cycle};

use colored::{Color, Colorize};

use super::TaskAndDeps;
use crate::workspace::{TaskInfo, TaskRef};

pub fn build_command_outputs(tasks: &[&TaskInfo]) -> HashMap<TaskRef, CommandOutput> {
    let max_len = CommandOutput::max_task_len(tasks);
    let mut colors = [
        Color::Blue,
        Color::Red,
        Color::Green,
        Color::Yellow,
        Color::Magenta,
        Color::Cyan,
        Color::BrightBlue,
        Color::BrightRed,
        Color::BrightGreen,
        Color::BrightYellow,
        Color::BrightMagenta,
        Color::BrightCyan,
    ]
    .iter()
    .cycle();

    tasks
        .iter()
        .map(|task| {
            (
                task.task_ref(),
                CommandOutput::new(
                    task,
                    max_len,
                    *colors
                        .next()
                        .expect("inifinite iterator to always return an item on next"),
                ),
            )
        })
        .collect()
}

pub struct CommandOutput {
    stdout: AnnotatedWrite<std::io::Stdout>,
    stdin: AnnotatedWrite<std::io::Stdin>,
}

impl CommandOutput {
    fn new(task: &TaskInfo, max_task_len: usize, color: Color) -> CommandOutput {
        let unaligned_annotation = CommandOutput::raw_task_annotation(task);

        let annotation = format!("{:>width$} ", unaligned_annotation, width = max_task_len)
            .color(color)
            .to_string();

        CommandOutput {
            stdout: AnnotatedWrite::new(annotation.clone(), std::io::stdout()),
            stdin: AnnotatedWrite::new(annotation, std::io::stdin()),
        }
    }

    fn max_task_len(tasks: &[&TaskInfo]) -> usize {
        tasks
            .iter()
            .map(|task| CommandOutput::raw_task_annotation(task).len())
            .max()
            .unwrap_or_default()
    }

    fn raw_task_annotation(task: &TaskInfo) -> String {
        format!("{} | {}", task.project.name(), task.name)
    }

    // TODO: Make this async, also maybe make it return a result
    pub fn stdout(&mut self, buf: &[u8]) {
        self.stdout
            .write_all(buf)
            .expect("Writing to stdout not to fail (TODO: remove this assumption)")
    }

    pub fn stderr(&mut self, buf: &[u8]) {
        self.stdout
            .write_all(buf)
            .expect("Writing to stdout not to fail (TODO: remove this assumption)")
    }
}

struct AnnotatedWrite<W> {
    annotation: String,
    inner: W,
    next_needs_annotated: bool,
    newline: u8,
}

impl<W> AnnotatedWrite<W> {
    fn new<'a>(annotation: impl Into<String>, inner: W) -> AnnotatedWrite<W> {
        AnnotatedWrite {
            annotation: annotation.into(),
            inner,
            next_needs_annotated: true,
            newline: u8::try_from('\n').unwrap(),
        }
    }
}

impl<W> std::io::Write for AnnotatedWrite<W>
where
    W: std::io::Write,
{
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut bytes_used = 0;
        if self.next_needs_annotated {
            // TODO: Write annotation.
            self.next_needs_annotated = false;
            self.inner.write_all(self.annotation.as_bytes())?;
        }
        let mut chunks = buf.split_inclusive(|c| *c == self.newline).peekable();
        while let Some(chunk) = chunks.next() {
            bytes_used += self.inner.write(chunk)?;
            if chunks.peek().is_some() {
                self.inner.write_all(self.annotation.as_bytes())?;
            } else if chunk.ends_with(&[self.newline]) {
                self.next_needs_annotated = true;
            }
        }
        Ok(bytes_used)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}
