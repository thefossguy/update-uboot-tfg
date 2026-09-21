// SPDX-License-Identifier: GPL-2.0-only
// Copyright (C) 2026 Pratham Patel <prathampatel@thefossguy.com>
//
// This program is free software; you can redistribute it and/or
// modify it under the terms of the GNU General Public License
// as published by the Free Software Foundation; only as version 2
// of the License, NOT as a later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program; if not, see <https://www.gnu.org/licenses/>.

use std::error::Error;
use std::fmt;
use std::path::Path;
use std::process::Command;

use tfg_helpers::command_helpers::CommandOutputStatus;
use tfg_helpers::{get_process_stdout, log_then_output, return_stderr_as_err};

#[derive(Clone)]
pub enum SupportedBoards {
    RaspberryPi4ModelB,

    RaspberryPi5ModelB,

    CM3588,
    NanoPCT6,
    OrangePi5,
    Rock5ModelB,
}
impl SupportedBoards {
    fn from(specified_board: &str) -> Result<Self, Box<dyn Error>> {
        match specified_board {
            "raspberry-pi-4-model-b" => Ok(Self::RaspberryPi4ModelB),
            "raspberry-pi-5-model-b" => Ok(Self::RaspberryPi5ModelB),
            "cm3588" => Ok(Self::CM3588),
            "nanopc-t6" => Ok(Self::NanoPCT6),
            "orange-pi-5" => Ok(Self::OrangePi5),
            "rock-5-model-b" => Ok(Self::Rock5ModelB),
            _ => {
                Err(format!("The specified board '{specified_board}' is not yet supported").into())
            }
        }
    }
}
impl fmt::Debug for SupportedBoards {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let debug_str = match self {
            Self::RaspberryPi4ModelB => "raspberry-pi-4-model-b",
            Self::RaspberryPi5ModelB => "raspberry-pi-5-model-b",
            Self::CM3588 => "cm3588",
            Self::NanoPCT6 => "nanopc-t6",
            Self::OrangePi5 => "orange-pi-5",
            Self::Rock5ModelB => "rock-5-model-b",
        };
        write!(f, "{debug_str}")
    }
}
impl fmt::Display for SupportedBoards {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Debug)]
pub struct UbootConfig {
    pub uboot_path: String,
}
impl UbootConfig {
    fn from(
        board: Option<SupportedBoards>,
        specified_uboot_path: String,
    ) -> Result<Self, Box<dyn Error>> {
        let Some(board) = board else {
            return Err("`--board` must be specified before `--uboot-path`".into());
        };

        let specified_uboot_path_path = Path::new(&specified_uboot_path);
        if !specified_uboot_path_path.exists() {
            return Err(format!(
                "The specified uboot path '{specified_uboot_path}' does not exist"
            )
            .into());
        }

        match board {
            SupportedBoards::RaspberryPi4ModelB | SupportedBoards::RaspberryPi5ModelB => {
                if specified_uboot_path_path.is_dir() {
                    Ok(Self {
                        uboot_path: specified_uboot_path,
                    })
                } else {
                    Err("The board '{board}' requires `--uboot-path` to be a directory".into())
                }
            }

            SupportedBoards::CM3588 => {
                if specified_uboot_path_path.is_dir() {
                    let specified_uboot_bin_path =
                        specified_uboot_path_path.join("u-boot-rockchip.bin");
                    if specified_uboot_bin_path.is_file() {
                        Ok(Self {
                            uboot_path: specified_uboot_bin_path.display().to_string(),
                        })
                    } else {
                        Err(format!("`--uboot-path` was specified as '{specified_uboot_path}' but it does not contain the 'u-boot-rockchip.bin' file.").into())
                    }
                } else {
                    Ok(Self {
                        uboot_path: specified_uboot_path,
                    })
                }
            }
            SupportedBoards::NanoPCT6
            | SupportedBoards::OrangePi5
            | SupportedBoards::Rock5ModelB => {
                if specified_uboot_path_path.is_dir() {
                    let specified_uboot_bin_path =
                        specified_uboot_path_path.join("u-boot-rockchip-spi.bin");
                    if specified_uboot_bin_path.is_file() {
                        Ok(Self {
                            uboot_path: specified_uboot_bin_path.display().to_string(),
                        })
                    } else {
                        Err(format!("`--uboot-path` was specified as '{specified_uboot_path}' but it does not contain the 'u-boot-rockchip-spi.bin' file.").into())
                    }
                } else {
                    Ok(Self {
                        uboot_path: specified_uboot_path,
                    })
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct VFATFileSystem {
    pub block_dev: String,
    pub mount_point: String,
}
impl VFATFileSystem {
    fn new() -> Result<Self, Box<dyn Error>> {
        let mut findmnt_cmd = Command::new("findmnt");
        findmnt_cmd.args([
            "--types",
            "vfat",
            "--raw",
            "--noheadings",
            "--output",
            "SOURCE,TARGET",
        ]);
        let findmnt_process_result = log_then_output!(findmnt_cmd);
        if !findmnt_process_result.was_process_successful() {
            return return_stderr_as_err!(
                "The `findmnt` command to determine the vFAT partition failed",
                findmnt_process_result
            );
        }

        let findmnt_process_output = findmnt_process_result?;
        let findmnt_process_stdout = get_process_stdout!(findmnt_process_output);
        let (partition, mount_point) = if findmnt_process_stdout.lines().collect::<Vec<_>>().len()
            == 1
        {
            let split_findmnt_process_stdout =
                findmnt_process_stdout.split(' ').collect::<Vec<&str>>();
            let Some(device) = split_findmnt_process_stdout.first() else {
                return Err(
                    "Could not determine the partition device of the vFAT filesystem".into(),
                );
            };
            let Some(mount_point) = split_findmnt_process_stdout.get(1) else {
                return Err("Could not determine the mount point of the vFAT filesystem".into());
            };
            (device.to_string(), mount_point.to_string())
        } else {
            return Err(format!("Expected a single line in `findmnt` process' stdout, encountered: {findmnt_process_stdout}").into());
        };

        let mut lsblk_cmd = Command::new("lsblk");
        lsblk_cmd.args(["--noheadings", "--output", "pkname", &partition]);
        let lsblk_process_result = log_then_output!(lsblk_cmd);
        if !lsblk_process_result.was_process_successful() {
            return return_stderr_as_err!(
                "The `lsblk` command to determine the parent block device failed",
                lsblk_process_result
            );
        }
        let lsblk_process_output = lsblk_process_result?;
        let lsblk_process_stdout = get_process_stdout!(lsblk_process_output);
        let block_dev = format!("/dev/{lsblk_process_stdout}");

        Ok(Self {
            block_dev,
            mount_point,
        })
    }
}

#[derive(Debug)]
pub struct UpdateUbootConfig {
    pub board: SupportedBoards,
    pub uboot_config: UbootConfig,
    pub vfat_filesystem: VFATFileSystem,
}

pub fn configure() -> Result<UpdateUbootConfig, Box<dyn Error>> {
    use lexopt::prelude::*;

    let mut board = None;
    let mut uboot_config = None;
    let mut new_uboot_version = None;

    let mut lexopt_parser = lexopt::Parser::from_env();
    while let Some(arg) = lexopt_parser.next()? {
        match arg {
            Long("board") => {
                board = Some(SupportedBoards::from(&lexopt_parser.value()?.string()?)?);
            }
            Long("uboot-path") => {
                uboot_config = Some(UbootConfig::from(
                    board.clone(),
                    lexopt_parser.value()?.string()?,
                )?);
            }
            Long("--uboot-version") => {
                let specified_new_uboot_version = lexopt_parser.value()?.string()?;
                if specified_new_uboot_version.is_empty() {
                    return Err("`--uboot-version` cannot be empty".into());
                }
                new_uboot_version = Some(specified_new_uboot_version);
            }
            _ => return Err(arg.unexpected().into()),
        }
    }

    let Some(new_uboot_version) = new_uboot_version else {
        return Err("`--uboot-version` is required".into());
    };
    let Ok(current_uboot_version) =
        std::fs::read_to_string("/proc/device-tree/chosen/u-boot,version")
    else {
        return Err("Could not determine the current U-Boot version".into());
    };
    if current_uboot_version == new_uboot_version {
        eprintln!(
            "Current U-Boot version '{current_uboot_version}' and new U-Boot version '{new_uboot_version}' are the same, nothing to do."
        );
        std::process::exit(0);
    }

    let Some(board) = board else {
        return Err("`--board` is required".into());
    };
    let Some(uboot_config) = uboot_config else {
        return Err("`--uboot-path` is required".into());
    };

    Ok(UpdateUbootConfig {
        board,
        uboot_config,
        vfat_filesystem: VFATFileSystem::new()?,
    })
}
