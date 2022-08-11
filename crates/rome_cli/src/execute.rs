use crate::traversal::traverse;
use crate::{CliSession, Termination};
use rome_console::{markup, ConsoleExt};
use rome_fs::RomePath;
use rome_service::workspace::{
    FeatureName, FixFileMode, FormatFileParams, OpenFileParams, SupportsFeatureParams,
};
use std::path::PathBuf;

#[derive(Clone)]
pub(crate) enum ExecutionMode {
    /// This mode is enabled when running the command `rome check`
    Check {
        /// The maximum number of diagnostics that can be printed in console
        max_diagnostics: u16,

        /// The type of fixes that should be applied when analyzing a file.
        ///
        /// It's [None] if the `check` command is called without `--apply` or `--apply-suggested`
        /// arguments.
        fix_file_mode: Option<FixFileMode>,
    },
    /// This mode is enabled when running the command `rome ci`
    CI,
    /// This mode is enabled when running the command `rome format`
    Format {
        /// It ignores parse errors
        ignore_errors: bool,
        /// It writes the new content on file
        write: bool,
        /// An optional tuple.
        /// 1. The virtual path to the file
        /// 2. The content of the file
        stdin: Option<(PathBuf, String)>,
    },
}

impl ExecutionMode {
    pub(crate) fn get_max_diagnostics(&self) -> Option<u16> {
        match self {
            ExecutionMode::Check {
                max_diagnostics, ..
            } => Some(*max_diagnostics),
            _ => None,
        }
    }

    /// `true` only when running the traversal in [TraversalMode::Check] and `should_fix` is `true`
    pub(crate) fn as_fix_file_mode(&self) -> Option<&FixFileMode> {
        if let ExecutionMode::Check { fix_file_mode, .. } = self {
            fix_file_mode.as_ref()
        } else {
            None
        }
    }

    pub(crate) fn is_ci(&self) -> bool {
        matches!(self, ExecutionMode::CI { .. })
    }

    pub(crate) fn is_check(&self) -> bool {
        matches!(self, ExecutionMode::Check { .. })
    }

    pub(crate) fn is_format(&self) -> bool {
        matches!(self, ExecutionMode::Format { .. })
    }

    pub(crate) fn as_stdin_file(&self) -> Option<&(PathBuf, String)> {
        match self {
            ExecutionMode::Format { stdin, .. } => stdin.as_ref(),
            _ => None,
        }
    }
}

/// Based on the [mode](ExecutionMode), the function might launch a traversal of the file system
/// or handles the stdin file.
pub(crate) fn execute_mode(
    mode: ExecutionMode,
    mut session: CliSession,
) -> Result<(), Termination> {
    // don't do any traversal if there's some content coming from stdin
    if let Some((path, content)) = mode.as_stdin_file() {
        let workspace = &*session.app.workspace;
        let console = &mut *session.app.console;
        let rome_path = RomePath::new(path, 0);

        if mode.is_format() {
            let can_format = workspace.supports_feature(SupportsFeatureParams {
                path: rome_path.clone(),
                feature: FeatureName::Format,
            });
            if can_format {
                workspace.open_file(OpenFileParams {
                    path: rome_path.clone(),
                    version: 0,
                    content: content.into(),
                })?;
                let printed = workspace.format_file(FormatFileParams { path: rome_path })?;

                console.log(markup! {
                    {printed.as_code()}
                });
            } else {
                console.log(markup! {
                    {content}
                });
                console.error(markup!{
                    <Warn>"The content was not formatted because the formatter is currently disabled."</Warn>
                })
            }
        }

        Ok(())
    } else {
        traverse(mode, session)
    }
}
