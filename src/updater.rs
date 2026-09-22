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

use crate::config::{SupportedBoards, UpdateUbootConfig};

use std::error::Error;
use std::fs;
use std::os::unix::fs::FileTypeExt;
use std::path::PathBuf;
use std::process::Command;

use tfg_helpers::command_helpers::CommandOutputStatus;
use tfg_helpers::log_then_status;

fn copy_dir_recursive(source: &PathBuf, destination: &PathBuf) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(destination)?;

    for entry in fs::read_dir(source)? {
        let dir_entry = entry?;
        let dir_entry_path = dir_entry.path();
        let destination_path = destination.join(&dir_entry_path);

        if dir_entry_path.is_dir() {
            copy_dir_recursive(&dir_entry_path, &destination_path)?;
        } else {
            fs::copy(&dir_entry_path, &destination_path)?;
        }
    }

    Ok(())
}

fn update_uboot_by_copying_files_to_efi_part(
    update_uboot_config: &UpdateUbootConfig,
) -> Result<(), Box<dyn Error>> {
    let source = PathBuf::from(&update_uboot_config.uboot_config.uboot_path);
    let destination = PathBuf::from(&update_uboot_config.vfat_filesystem.mount_point);
    copy_dir_recursive(&source, &destination)
}

fn update_uboot_by_flashing_to_dev_mtd0(
    update_uboot_config: &UpdateUbootConfig,
) -> Result<(), Box<dyn Error>> {
    if let Ok(dev_mtd0_metadata) = fs::metadata("/dev/mtd0")
        && dev_mtd0_metadata.file_type().is_char_device()
    {
        let mut flashcp_cmd = Command::new("flashcp");
        flashcp_cmd.args([
            "-v",
            &update_uboot_config.uboot_config.uboot_path,
            "/dev/mtd0",
        ]);
        let flashcp_process_result = log_then_status!(flashcp_cmd);
        if flashcp_process_result.was_process_successful() {
            Ok(())
        } else {
            Err("The `flashcp` command failed".into())
        }
    } else {
        Err("Either '/dev/mtd0' doesn't exist or is not a character device".into())
    }
}

fn update_uboot_by_writing_to_efi_part_block_dev(
    update_uboot_config: &UpdateUbootConfig,
) -> Result<(), Box<dyn Error>> {
    let vfat_filesystem_block_dev = &update_uboot_config.vfat_filesystem.block_dev;
    if let Ok(block_dev_metadata) = fs::metadata(vfat_filesystem_block_dev)
        && block_dev_metadata.file_type().is_block_device()
    {
        let output_file = format!("of={vfat_filesystem_block_dev}");
        let input_file = format!("if={}", update_uboot_config.uboot_config.uboot_path);
        let mut dd_cmd = Command::new("dd");
        dd_cmd.args([
            &output_file,
            &input_file,
            "bs=512",
            "seek=64",
            "conv=notrunc,fsync",
        ]);
        let dd_process_result = log_then_status!(dd_cmd);
        if dd_process_result.was_process_successful() {
            Ok(())
        } else {
            Err("The `dd` command to flash u-boot failed".into())
        }
    } else {
        Err(format!("Either the block device '{vfat_filesystem_block_dev}' ").into())
    }
}

pub fn update_uboot(update_uboot_config: &UpdateUbootConfig) -> Result<(), Box<dyn Error>> {
    match update_uboot_config.board {
        SupportedBoards::RaspberryPi4ModelB | SupportedBoards::RaspberryPi5ModelB => {
            update_uboot_by_copying_files_to_efi_part(update_uboot_config)
        }

        SupportedBoards::CM3588 => {
            update_uboot_by_writing_to_efi_part_block_dev(update_uboot_config)
        }
        SupportedBoards::NanoPCT6 | SupportedBoards::OrangePi5 | SupportedBoards::Rock5ModelB => {
            update_uboot_by_flashing_to_dev_mtd0(update_uboot_config)
        }
    }
}
