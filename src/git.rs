use std::path::PathBuf;

use thiserror::Error;

pub type Commit = String;
pub type Command = Vec<String>;

pub enum Mode {
    Feature(String), // base branch, e.g. 'main'
    Main(String),    // base commit, e.g. 'HEAD^1'
}

#[derive(PartialEq, Eq, Error, Debug, miette::Diagnostic)]
pub enum GitError {
    #[error("Cannot find merge base with branch {0}: {1}")]
    MergeBase(String, String), // base branch, error
    #[error("Finding changed files failed: {0}")]
    Diff(String), // error
    #[error("Error finding the repository root: {0}")]
    CouldntFindRoot(String), // error
}

pub fn repo_root() -> Result<PathBuf, GitError> {
    real_impl().repo_root().map(PathBuf::from)
}

pub fn files_changed(mode: Mode) -> Result<Vec<PathBuf>, GitError> {
    let paths = real_impl().diff(mode, vec![])?;

    Ok(paths.into_iter().map(PathBuf::from).collect())
}

pub fn have_files_changed(since: String, path: camino::Utf8PathBuf) -> Result<bool, GitError> {
    let paths = real_impl().diff(Mode::Main(since), vec![path.to_string()])?;

    Ok(!paths.is_empty())
}

fn real_impl() -> GitImpl<impl FnMut(Command) -> Result<String, String>> {
    GitImpl::new(|command| {
        let prog = &command[0];
        let args = &command[1..];

        let out = std::process::Command::new(prog)
            .args(args)
            .output()
            .map_err(|e| format!("Git call failed: {}", e))?;

        if out.status.success() {
            std::str::from_utf8(&out.stdout)
                .map(|s| s.to_string())
                .map_err(|e| format!("Could not convert git output to string: {}", e))
        } else {
            let error = std::str::from_utf8(&out.stderr)
                .map_err(|e| format!("Could not convert git output to string: {}", e))?;

            Err(error.to_string())
        }
    })
}

struct GitImpl<Executor>
where
    Executor: FnMut(Command) -> Result<String, String>,
{
    // Inversion of control for command execution to make Git pure
    // and easier to test
    executor: Executor,
}

impl<Executor> GitImpl<Executor>
where
    Executor: FnMut(Command) -> Result<String, String>,
{
    fn new(executor: Executor) -> Self {
        Self { executor }
    }

    fn repo_root(&mut self) -> Result<String, GitError> {
        self.execute(["git", "rev-parse", "--show-toplevel"])
            .map(|r| r.trim().to_string())
            .map_err(GitError::CouldntFindRoot)
    }

    fn diff_base(&mut self, mode: Mode) -> Result<Commit, GitError> {
        match mode {
            Mode::Feature(base_branch) => self
                .execute(["git", "merge-base", base_branch.as_ref(), "HEAD"])
                .map(|base| base.trim_end().to_string())
                .map_err(|e| GitError::MergeBase(base_branch, e)),
            Mode::Main(base_commit) => Ok(base_commit.trim_end().to_string()),
        }
    }

    fn diff(&mut self, mode: Mode, files: Vec<String>) -> Result<Vec<String>, GitError> {
        let base = self.diff_base(mode)?;

        let mut command = vec![
            "git",
            "diff",
            "--no-commit-id",
            "--name-only",
            "-r",
            base.as_ref(),
        ];

        if !files.is_empty() {
            command.push("--");
            command.extend(files.iter().map(|s| -> &str { s.as_ref() }));
        }

        self.execute(command)
            .map(|files| {
                files
                    .trim_end()
                    .split('\n')
                    .map(|f| f.to_string())
                    .collect()
            })
            .map_err(GitError::Diff)
    }

    fn execute<'a>(
        &mut self,
        command: impl IntoIterator<Item = &'a str>,
    ) -> Result<String, String> {
        (self.executor)(command.into_iter().map(|p| p.to_string()).collect())
    }
}

#[cfg(test)]
mod test {
    mod diff_base {
        use super::super::*;

        #[test]
        fn base_on_feature_branch() {
            let mut actual_command: Option<Command> = None;
            let expected_command = Some(vec![
                "git".into(),
                "merge-base".into(),
                "main".into(),
                "HEAD".into(),
            ]);

            let mock_exec = |cmd: Command| -> Result<String, String> {
                actual_command = Some(cmd);

                Ok("abc\n".to_string()) // check new line is trimmed
            };

            let mut git = GitImpl::new(mock_exec);

            let actual = git.diff_base(Mode::Feature("main".to_string()));
            let expected = Ok("abc".to_string());

            assert_eq!(actual, expected);
            assert_eq!(actual_command, expected_command);
        }

        #[test]
        fn base_on_main_branch() {
            let mut actual_command: Option<Command> = None;
            let expected_command = None;

            let mock_exec = |cmd: Command| -> Result<String, String> {
                actual_command = Some(cmd);

                Ok("abc\n".to_string())
            };

            let mut git = GitImpl::new(mock_exec);

            let actual = git.diff_base(Mode::Main("HEAD^1".to_string()));
            let expected = Ok("HEAD^1".to_string());

            assert_eq!(actual, expected);
            assert_eq!(actual_command, expected_command);
        }
    }

    mod diff {
        use super::super::*;

        #[test]
        fn diff_on_feature_branch() {
            let mut actual_commands: Vec<Command> = vec![];
            let expected_command: Vec<String> = vec![
                "git".into(),
                "diff".into(),
                "--no-commit-id".into(),
                "--name-only".into(),
                "-r".into(),
                "main".into(),
            ];

            let mock_exec = |cmd: Command| -> Result<String, String> {
                actual_commands.push(cmd);

                if actual_commands.len() < 2 {
                    Ok("main\n".to_string())
                } else {
                    Ok("one\ntwo\nthree\n".to_string())
                }
            };

            let mut git = GitImpl::new(mock_exec);

            let actual = git.diff(Mode::Feature("main".to_string()), vec![]);
            let expected = Ok(vec![
                "one".to_string(),
                "two".to_string(),
                "three".to_string(),
            ]);

            assert_eq!(actual, expected);
            assert_eq!(actual_commands[1], expected_command);
        }

        #[test]
        fn diff_on_main_branch() {
            let mut actual_commands: Vec<Command> = vec![];
            let expected_command: Vec<String> = vec![
                "git".into(),
                "diff".into(),
                "--no-commit-id".into(),
                "--name-only".into(),
                "-r".into(),
                "HEAD^1".into(),
            ];

            let mock_exec = |cmd: Command| -> Result<String, String> {
                actual_commands.push(cmd);

                Ok("one\ntwo\nthree\n".to_string())
            };

            let mut git = GitImpl::new(mock_exec);

            let actual = git.diff(Mode::Main("HEAD^1".to_string()), vec![]);
            let expected = Ok(vec![
                "one".to_string(),
                "two".to_string(),
                "three".to_string(),
            ]);

            assert_eq!(actual, expected);
            assert_eq!(actual_commands[0], expected_command);
        }

        #[test]
        fn diff_with_files() {
            let mut actual_commands: Vec<Command> = vec![];
            let expected_command: Vec<String> = vec![
                "git".into(),
                "diff".into(),
                "--no-commit-id".into(),
                "--name-only".into(),
                "-r".into(),
                "HEAD^1".into(),
                "--".into(),
                "blah.txt".into(),
                "blah2.txt".into(),
            ];

            let mock_exec = |cmd: Command| -> Result<String, String> {
                actual_commands.push(cmd);

                Ok("one\ntwo\nthree\n".to_string())
            };

            let mut git = GitImpl::new(mock_exec);

            let actual = git.diff(
                Mode::Main("HEAD^1".to_string()),
                vec!["blah.txt".into(), "blah2.txt".into()],
            );
            let expected = Ok(vec![
                "one".to_string(),
                "two".to_string(),
                "three".to_string(),
            ]);

            assert_eq!(actual, expected);
            assert_eq!(actual_commands[0], expected_command);
        }
    }
}
