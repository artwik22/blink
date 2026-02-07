use std::fs;
use std::path::PathBuf;

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct DriveInfo {
    pub name: String,
    pub mount_point: PathBuf,
    pub device: String,
    pub fs_type: String,
    pub total_size: u64,
    pub available_size: u64,
    pub icon_name: String,
}

impl DriveInfo {
    pub fn size_display(&self) -> String {
        if self.total_size == 0 {
            return String::new();
        }

        const GB: u64 = 1024 * 1024 * 1024;
        let available_gb = self.available_size as f64 / GB as f64;
        let total_gb = self.total_size as f64 / GB as f64;

        format!("{:.1} GB free of {:.1} GB", available_gb, total_gb)
    }
}

pub struct DriveScanner;

impl DriveScanner {
    pub fn scan() -> Vec<DriveInfo> {
        let mut drives = Vec::new();

        // Read /proc/mounts for mounted filesystems
        if let Ok(content) = fs::read_to_string("/proc/mounts") {
            for line in content.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 4 {
                    continue;
                }

                let device = parts[0];
                let mount_point = parts[1];
                let fs_type = parts[2];

                // Filter out virtual filesystems and system mounts
                if Self::should_show_mount(device, mount_point, fs_type) {
                    let mount_path = PathBuf::from(mount_point);
                    
                    // Get disk usage
                    let (total, available) = Self::get_disk_usage(&mount_path);

                    let name = Self::get_drive_name(device, mount_point);
                    let icon = Self::get_drive_icon(device, fs_type, mount_point);

                    drives.push(DriveInfo {
                        name,
                        mount_point: mount_path,
                        device: device.to_string(),
                        fs_type: fs_type.to_string(),
                        total_size: total,
                        available_size: available,
                        icon_name: icon,
                    });
                }
            }
        }

        // Sort by mount point
        drives.sort_by(|a, b| a.mount_point.cmp(&b.mount_point));
        drives
    }

    fn should_show_mount(device: &str, mount_point: &str, fs_type: &str) -> bool {
        // Skip virtual filesystems
        let virtual_fs = [
            "sysfs", "proc", "devtmpfs", "devpts", "tmpfs", "securityfs",
            "cgroup", "cgroup2", "pstore", "bpf", "debugfs", "tracefs",
            "configfs", "fusectl", "hugetlbfs", "mqueue", "ramfs",
            "autofs", "rpc_pipefs", "nfsd", "overlay", "nsfs", "fuse.portal",
        ];

        if virtual_fs.contains(&fs_type) {
            return false;
        }

        // Skip system directories
        let skip_mounts = [
            "/sys", "/proc", "/dev", "/run", "/snap", "/boot/efi",
            "/var/lib", "/var/cache",
        ];

        for skip in &skip_mounts {
            if mount_point.starts_with(skip) {
                return false;
            }
        }

        // Only show real block devices or network mounts
        device.starts_with("/dev/") 
            || device.contains(":/")  // NFS/network
            || fs_type == "nfs" 
            || fs_type == "nfs4"
            || fs_type == "cifs"
            || fs_type == "smb"
            || mount_point.starts_with("/media/")
            || mount_point.starts_with("/mnt/")
            || mount_point == "/"
    }

    fn get_drive_name(device: &str, mount_point: &str) -> String {
        // For root filesystem
        if mount_point == "/" {
            return String::from("System");
        }

        // For /home
        if mount_point == "/home" {
            return String::from("Home");
        }

        // Get label from mount point name
        if let Some(name) = mount_point.rsplit('/').next() {
            if !name.is_empty() {
                return name.to_string();
            }
        }

        // Fallback to device name
        if let Some(name) = device.rsplit('/').next() {
            return name.to_string();
        }

        String::from("Disk")
    }

    fn get_drive_icon(device: &str, fs_type: &str, mount_point: &str) -> String {
        // Network drives
        if device.contains(":/") || fs_type == "nfs" || fs_type == "cifs" || fs_type == "smb" {
            return String::from("folder-remote");
        }

        // USB/removable media (usually mounted in /media or /run/media)
        if mount_point.contains("/media/") || mount_point.contains("/run/media/") {
            return String::from("drive-removable-media");
        }

        // SD cards
        if device.contains("mmcblk") {
            return String::from("media-flash-sd-mmc");
        }

        // NVMe drives
        if device.contains("nvme") {
            return String::from("drive-harddisk-solidstate");
        }

        // Root filesystem
        if mount_point == "/" {
            return String::from("drive-harddisk-system");
        }

        String::from("drive-harddisk")
    }

    fn get_disk_usage(path: &PathBuf) -> (u64, u64) {
        // Use df command to get disk usage
        if let Ok(output) = std::process::Command::new("df")
            .arg("-B1")
            .arg(path)
            .output()
        {
            if let Ok(stdout) = String::from_utf8(output.stdout) {
                let lines: Vec<&str> = stdout.lines().collect();
                if lines.len() >= 2 {
                    let parts: Vec<&str> = lines[1].split_whitespace().collect();
                    if parts.len() >= 4 {
                        let total = parts[1].parse::<u64>().unwrap_or(0);
                        let available = parts[3].parse::<u64>().unwrap_or(0);
                        return (total, available);
                    }
                }
            }
        }

        (0, 0)
    }
}
