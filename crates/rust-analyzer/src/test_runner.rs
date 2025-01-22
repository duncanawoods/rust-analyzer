//! This module provides the functionality needed to run `cargo test` in a background
//! thread and report the result of each test in a channel.

use crossbeam_channel::Sender;
use paths::AbsPath;
use project_model::TargetKind;
use serde::Deserialize as _;
use serde_derive::Deserialize;
use toolchain::Tool;

use crate::{
    command::{CommandHandle, ParseFromLine},
    flycheck::CargoOptions,
};

#[derive(Debug, Deserialize)]
#[serde(tag = "event", rename_all = "camelCase")]
pub(crate) enum TestState {
    Started,
    Ok,
    Ignored,
    Failed {
        // the stdout field is not always present depending on cargo test flags
        #[serde(skip_serializing_if = "String::is_empty", default)]
        stdout: String,
    },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub(crate) enum CargoTestMessage {
    Test {
        name: String,
        #[serde(flatten)]
        state: TestState,
    },
    Suite,
    Finished,
    Custom {
        text: String,
    },
}

impl ParseFromLine for CargoTestMessage {
    fn from_line(line: &str, _: &mut String) -> Option<Self> {
        let mut deserializer = serde_json::Deserializer::from_str(line);
        deserializer.disable_recursion_limit();
        if let Ok(message) = CargoTestMessage::deserialize(&mut deserializer) {
            return Some(message);
        }

        Some(CargoTestMessage::Custom { text: line.to_owned() })
    }

    fn from_eof() -> Option<Self> {
        Some(CargoTestMessage::Finished)
    }
}

#[derive(Debug)]
pub(crate) struct CargoTestHandle {
    _handle: CommandHandle<CargoTestMessage>,
}

// Example of a cargo test command:
// cargo test --workspace --no-fail-fast -- -Z unstable-options --format=json
// or
// cargo test --package my-package --no-fail-fast -- module::func -Z unstable-options --format=json

#[derive(Debug)]
pub(crate) enum TestTarget {
    Workspace,
    Package { package: String, target: String, kind: TargetKind },
}

pub(crate) enum TestToolKind {
    CargoTest,
    CargoNextest,
}

impl CargoTestHandle {
    pub(crate) fn new(
        test_tool: TestToolKind,
        path: Option<&str>,
        options: CargoOptions,
        root: &AbsPath,
        test_target: TestTarget,
        sender: Sender<CargoTestMessage>,
    ) -> std::io::Result<Self> {
        let mut cmd = toolchain::command(Tool::Cargo.path(), root);
        cmd.env("RUSTC_BOOTSTRAP", "1");

        match test_tool {
            TestToolKind::CargoTest => {
                cmd.arg("test");

                match &test_target {
                    TestTarget::Package { package, target, kind } => {
                        cmd.arg("--package");
                        cmd.arg(package);
                        match kind {
                            TargetKind::Lib { .. } => {
                                cmd.arg("--lib");
                                // no name required because there can only be one lib target
                            }
                            TargetKind::Other => {
                                // unsupported by cargo
                            }
                            _ => {
                                cmd.arg(format!("--{kind}"));
                                cmd.arg(target);
                            }
                        }
                    }
                    TestTarget::Workspace => {
                        cmd.arg("--workspace");
                    }
                }
            }
            TestToolKind::CargoNextest => {
                cmd.env("NEXTEST_EXPERIMENTAL_LIBTEST_JSON", "1");
                cmd.arg("nextest");
                cmd.arg("run");
                if let Some(dsl) = path {
                    cmd.arg("-E");
                    cmd.arg(dsl);
                }
            }
        }

        // --no-fail-fast is needed to ensure that all requested tests will run
        cmd.arg("--no-fail-fast");
        cmd.arg("--manifest-path");
        cmd.arg(root.join("Cargo.toml"));
        options.apply_on_command(&mut cmd);

        match test_tool {
            TestToolKind::CargoTest => {
                cmd.arg("--");
                if let Some(path) = path {
                    cmd.arg(path);
                }
                cmd.args(["-Z", "unstable-options"]);
                cmd.arg("--format=json");

                for extra_arg in options.extra_test_bin_args {
                    cmd.arg(extra_arg);
                }
            }
            TestToolKind::CargoNextest => {
                cmd.arg("--message-format");
                cmd.arg("libtest-json");
                cmd.arg("--");
            }
        }

        // TODO: quick logging to remove
        tracing::error!("\nTest command:\n{cmd:?}\n");

        Ok(Self { _handle: CommandHandle::spawn(cmd, sender)? })
    }
}
