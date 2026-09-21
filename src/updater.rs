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

fn update_uboot_raspberry_pi_4_model_b(
    update_uboot_config: &UpdateUbootConfig,
) -> Result<(), Box<dyn Error>> {
    update_uboot_by_copying_files_to_efi_part(update_uboot_config)
}

fn update_uboot_raspberry_pi_5_model_b(
    update_uboot_config: &UpdateUbootConfig,
) -> Result<(), Box<dyn Error>> {
    update_uboot_by_copying_files_to_efi_part(update_uboot_config)
}

fn flash_uboot_to_emmc(mmc_device: &str, uboot_bin: &str) -> Result<(), Box<dyn Error>> {
    let mut dd_cmd = Command::new("dd");
    dd_cmd.args([
        format!("of={mmc_device}").as_str(),
        format!("if={uboot_bin}").as_str(),
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
}

fn update_uboot_rk3588_family(
    update_uboot_config: &UpdateUbootConfig,
) -> Result<(), Box<dyn Error>> {
    if let Ok(dev_mtd0_metadata) = fs::metadata("/dev/mtd0")
        && dev_mtd0_metadata.file_type().is_char_device()
    {
        let mut flashcp_cmd = Command::new("flashcp");
        flashcp_cmd.args([
            "-v",
            &update_uboot_config.uboot_config.uboot_path,
            &update_uboot_config.vfat_filesystem.mount_point,
        ]);
        let flashcp_process_result = log_then_status!(flashcp_cmd);
        if flashcp_process_result.was_process_successful() {
            Ok(())
        } else {
            Err("The `flashcp` command failed".into())
        }
    } else if let Ok(dev_mmcblk0_metadata) = fs::metadata("/dev/mmcblk0")
        && dev_mmcblk0_metadata.file_type().is_block_device()
    {
        flash_uboot_to_emmc(
            &update_uboot_config.vfat_filesystem.block_dev,
            &update_uboot_config.uboot_config.uboot_path,
        )
    } else if let Ok(dev_mmcblk1_metadata) = fs::metadata("/dev/mmcblk1")
        && dev_mmcblk1_metadata.file_type().is_block_device()
    {
        flash_uboot_to_emmc(
            &update_uboot_config.vfat_filesystem.block_dev,
            &update_uboot_config.uboot_config.uboot_path,
        )
    } else {
        Err("Neither `/dev/mtd0`, `/dev/mmcblk0` nor `/dev/mmcblk1` exist; not sure what to do here".into())
    }
}

fn update_uboot_cm3588(update_uboot_config: &UpdateUbootConfig) -> Result<(), Box<dyn Error>> {
    update_uboot_rk3588_family(update_uboot_config)
}

fn update_uboot_nanopc_t6(update_uboot_config: &UpdateUbootConfig) -> Result<(), Box<dyn Error>> {
    update_uboot_rk3588_family(update_uboot_config)
}

fn update_uboot_orange_pi_5(update_uboot_config: &UpdateUbootConfig) -> Result<(), Box<dyn Error>> {
    update_uboot_rk3588_family(update_uboot_config)
}

fn update_uboot_rock_5_model_b(
    update_uboot_config: &UpdateUbootConfig,
) -> Result<(), Box<dyn Error>> {
    update_uboot_rk3588_family(update_uboot_config)
}

pub fn update_uboot(update_uboot_config: &UpdateUbootConfig) -> Result<(), Box<dyn Error>> {
    match update_uboot_config.board {
        SupportedBoards::RaspberryPi4ModelB => {
            update_uboot_raspberry_pi_4_model_b(update_uboot_config)
        }
        SupportedBoards::RaspberryPi5ModelB => {
            update_uboot_raspberry_pi_5_model_b(update_uboot_config)
        }

        SupportedBoards::CM3588 => update_uboot_cm3588(update_uboot_config),
        SupportedBoards::NanoPCT6 => update_uboot_nanopc_t6(update_uboot_config),
        SupportedBoards::OrangePi5 => update_uboot_orange_pi_5(update_uboot_config),
        SupportedBoards::Rock5ModelB => update_uboot_rock_5_model_b(update_uboot_config),
    }
}
