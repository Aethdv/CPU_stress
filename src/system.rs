const MIN_BUFFER_MB: usize = 32;
const RAM_SAFETY_FACTOR: f64 = 0.9;

pub fn detect_memory_size(multiplier: usize) -> usize {
    let num_cpus = num_cpus::get();

    if let Some(l3_mb) = detect_l3_cache() {
        let recommended = (l3_mb * multiplier).max(MIN_BUFFER_MB);

        if let Some(total_ram_mb) = get_total_system_ram_mb() {
            let total_allocation_mb = recommended * num_cpus;
            let max_safe_mb = ((total_ram_mb as f64) * RAM_SAFETY_FACTOR) as usize;

            if total_allocation_mb > max_safe_mb {
                let adjusted = (max_safe_mb / num_cpus).max(MIN_BUFFER_MB);
                eprintln!(
                    "[Auto-detect] L3 cache: {} MB → Calculated {} MB buffer per thread ({}x multiplier)",
                    l3_mb, recommended, multiplier
                );

                eprintln!(
                    "[Warning] Total allocation would be {} MB ({} threads × {} MB)",
                    total_allocation_mb, num_cpus, recommended
                );

                eprintln!(
                    "[Warning] Exceeds {}% of system RAM ({} MB total, {} MB limit)",
                    (RAM_SAFETY_FACTOR * 100.0) as usize,
                    total_ram_mb,
                    max_safe_mb
                );

                eprintln!(
                    "[Auto-detect] Reducing to {} MB per thread (total: {} MB)",
                    adjusted,
                    adjusted * num_cpus
                );
                return adjusted;
            }
        }

        eprintln!(
            "[Auto-detect] L3 cache: {} MB → Using {} MB buffer per thread ({}x multiplier)",
            l3_mb, recommended, multiplier
        );
        return recommended;
    }

    let base_heuristic = match num_cpus {
        1..=2 => 32,    // Old single/dual-core (Athlon, Pentium)
        3..=4 => 64,    // Older quad-core (Ryzen 3 1200, i5-7400)
        5..=8 => 128,   // Mainstream (Ryzen 5, i7)
        9..=16 => 192,  // High-end desktop (Ryzen 7, i9)
        17..=32 => 256, // HEDT (Threadripper, Xeon W)
        33..=64 => 512,
        65..=128 => 768,
        _ => 1024,
    };

    let scaled = ((base_heuristic as f64) * (multiplier as f64 / 4.0)) as usize;
    let heuristic_mb = scaled.max(MIN_BUFFER_MB);

    eprintln!(
        "[Auto-detect] L3 cache unknown → Using heuristic {} MB ({}x multiplier, {} CPUs)",
        heuristic_mb, multiplier, num_cpus
    );
    heuristic_mb
}

#[cfg(target_os = "linux")]
fn detect_l3_cache() -> Option<usize> {
    detect_l3_cache_linux()
}

#[cfg(target_os = "windows")]
fn detect_l3_cache() -> Option<usize> {
    detect_l3_cache_windows()
}

#[cfg(target_os = "macos")]
fn detect_l3_cache() -> Option<usize> {
    detect_l3_cache_macos()
}

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
fn detect_l3_cache() -> Option<usize> {
    None
}

#[cfg(target_os = "linux")]
fn detect_l3_cache_linux() -> Option<usize> {
    use std::fs;

    for index in 0..=10 {
        let level_path = format!("/sys/devices/system/cpu/cpu0/cache/index{}/level", index);
        let size_path = format!("/sys/devices/system/cpu/cpu0/cache/index{}/size", index);

        if let Ok(level) = fs::read_to_string(&level_path)
            && level.trim() == "3"
            && let Ok(size_str) = fs::read_to_string(&size_path)
            && let Some(mb) = parse_cache_size(&size_str)
        {
            return Some(mb);
        }
    }

    None
}

#[cfg(target_os = "windows")]
fn detect_l3_cache_windows() -> Option<usize> {
    use std::mem;

    use windows_sys::Win32::System::SystemInformation::{
        GetLogicalProcessorInformationEx,
        RelationCache,
        SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX,
    };

    unsafe {
        let mut buffer_size: u32 = 0;
        GetLogicalProcessorInformationEx(RelationCache, std::ptr::null_mut(), &mut buffer_size);

        if buffer_size == 0 {
            return None;
        }

        let mut buffer = vec![0u8; buffer_size as usize];
        let buffer_ptr = buffer.as_mut_ptr() as *mut SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX;

        if GetLogicalProcessorInformationEx(RelationCache, buffer_ptr, &mut buffer_size) == 0 {
            return None;
        }

        let mut offset = 0usize;
        while offset + mem::size_of::<SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX>()
            <= buffer_size as usize
        {
            let info = &*(buffer.as_ptr().add(offset)
                as *const SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX);

            if info.Relationship == RelationCache {
                let cache_info_ptr =
                    (info as *const SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX as usize
                        + mem::size_of::<u32>()
                        + mem::size_of::<u32>()) as *const CacheDescriptor;

                let cache = &*cache_info_ptr;

                if cache.Level == 3 {
                    let size_mb = cache.CacheSize / (1024 * 1024);
                    if size_mb > 0 {
                        return Some(size_mb as usize);
                    }
                }
            }

            offset += info.Size as usize;
        }
    }

    None
}

#[cfg(target_os = "windows")]
#[repr(C)]
struct CacheDescriptor {
    Level:         u8,
    Associativity: u8,
    LineSize:      u16,
    CacheSize:     u32,
    Type:          u32,
}

#[cfg(target_os = "macos")]
fn detect_l3_cache_macos() -> Option<usize> {
    // Prefer direct L3 keys if available (Intel Macs)
    if let Some(bytes) = sysctl_u64("hw.l3cachesize") {
        let mb = (bytes / (1024 * 1024)) as usize;
        if mb > 0 {
            return Some(mb);
        }
    }

    // Apple Silicon may provide per-perflevel L3 sizes
    for key in [
        "hw.perflevel0.l3cachesize",
        "hw.perflevel1.l3cachesize",
        "hw.perflevel2.l3cachesize",
    ] {
        if let Some(bytes) = sysctl_u64(key) {
            let mb = (bytes / (1024 * 1024)) as usize;
            if mb > 0 {
                return Some(mb);
            }
        }
    }

    // Fallback: take the largest non-zero cache entry from hw.cachesize (array)
    if let Some(vals) = sysctl_u64_vec("hw.cachesize") {
        if let Some(max_bytes) = vals.into_iter().max() {
            let mb = (max_bytes / (1024 * 1024)) as usize;
            if mb > 0 {
                return Some(mb);
            }
        }
    }

    None
}

fn parse_cache_size(s: &str) -> Option<usize> {
    let s = s.trim();

    if s.ends_with('K') || s.ends_with('k') {
        let kb: usize = s[..s.len() - 1].parse().ok()?;
        Some(kb / 1024)
    } else if s.ends_with('M') || s.ends_with('m') {
        s[..s.len() - 1].parse().ok()
    } else {
        let bytes: usize = s.parse().ok()?;
        Some(bytes / (1024 * 1024))
    }
}

fn get_total_system_ram_mb() -> Option<usize> {
    #[cfg(target_os = "linux")]
    {
        use std::fs;
        if let Ok(contents) = fs::read_to_string("/proc/meminfo") {
            for line in contents.lines() {
                if line.starts_with("MemTotal:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2
                        && let Ok(kb) = parts[1].parse::<usize>()
                    {
                        return Some(kb / 1024);
                    }
                }
            }
        }
        None
    }

    #[cfg(target_os = "windows")]
    {
        use std::mem;

        use windows_sys::Win32::System::SystemInformation::{
            GlobalMemoryStatusEx,
            MEMORYSTATUSEX,
        };

        unsafe {
            let mut mem_info: MEMORYSTATUSEX = mem::zeroed();
            mem_info.dwLength = mem::size_of::<MEMORYSTATUSEX>() as u32;

            if GlobalMemoryStatusEx(&mut mem_info) != 0 {
                let total_mb = (mem_info.ullTotalPhys / (1024 * 1024)) as usize;
                return Some(total_mb);
            }
        }
        None
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(bytes) = sysctl_u64("hw.memsize") {
            return Some((bytes / (1024 * 1024)) as usize);
        }
        None
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    {
        None
    }
}

#[cfg(target_os = "macos")]
fn sysctl_u64(name: &str) -> Option<u64> {
    use std::ffi::{CString, c_void};

    unsafe extern "C" {
        fn sysctlbyname(
            name: *const std::os::raw::c_char,
            oldp: *mut c_void,
            oldlenp: *mut usize,
            newp: *mut c_void,
            newlen: usize,
        ) -> std::os::raw::c_int;
    }

    unsafe {
        let c_name = CString::new(name).ok()?;
        let mut value: u64 = 0;
        let mut size = std::mem::size_of::<u64>();
        let ret = sysctlbyname(
            c_name.as_ptr(),
            &mut value as *mut _ as *mut c_void,
            &mut size,
            std::ptr::null_mut(),
            0,
        );
        if ret == 0 && size == std::mem::size_of::<u64>() {
            Some(value)
        } else {
            None
        }
    }
}

#[cfg(target_os = "macos")]
fn sysctl_u64_vec(name: &str) -> Option<Vec<u64>> {
    use std::ffi::{CString, c_void};

    unsafe extern "C" {
        fn sysctlbyname(
            name: *const std::os::raw::c_char,
            oldp: *mut c_void,
            oldlenp: *mut usize,
            newp: *mut c_void,
            newlen: usize,
        ) -> std::os::raw::c_int;
    }

    unsafe {
        let c_name = CString::new(name).ok()?;
        let mut size: usize = 0;

        // First call to get size
        if sysctlbyname(
            c_name.as_ptr(),
            std::ptr::null_mut(),
            &mut size,
            std::ptr::null_mut(),
            0,
        ) != 0
            || size == 0
        {
            return None;
        }

        // Second call to fill buffer
        let mut buf = vec![0u8; size];
        if sysctlbyname(
            c_name.as_ptr(),
            buf.as_mut_ptr() as *mut c_void,
            &mut size,
            std::ptr::null_mut(),
            0,
        ) != 0
        {
            return None;
        }

        let count = size / std::mem::size_of::<u64>();
        let mut out = Vec::with_capacity(count);
        let ptr = buf.as_ptr() as *const u64;
        for i in 0..count {
            out.push(*ptr.add(i));
        }
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cache_size() {
        assert_eq!(parse_cache_size("8192K"), Some(8));
        assert_eq!(parse_cache_size("16384K"), Some(16));
        assert_eq!(parse_cache_size("12M"), Some(12));
        assert_eq!(parse_cache_size("256M"), Some(256));
        assert_eq!(parse_cache_size("8388608"), Some(8));
    }

    #[test]
    fn test_detect_memory_size_enforces_minimum() {
        let size = detect_memory_size(4);
        assert!(size >= MIN_BUFFER_MB);
    }

    #[test]
    fn test_cross_platform_detection_doesnt_panic() {
        let _ = super::detect_l3_cache();
    }

    #[test]
    fn test_get_total_system_ram() {
        if let Some(ram_mb) = super::get_total_system_ram_mb() {
            assert!(ram_mb >= 512);
            assert!(ram_mb <= 67_108_864);
        }
    }

    #[test]
    fn test_ram_aware_memory_size() {
        let size = detect_memory_size(4);
        assert!(size >= MIN_BUFFER_MB);

        let num_cpus = num_cpus::get();
        let total = size * num_cpus;

        if let Some(ram_mb) = super::get_total_system_ram_mb() {
            let max_reasonable = ((ram_mb as f64) * RAM_SAFETY_FACTOR) as usize;
            assert!(
                total <= max_reasonable,
                "Total allocation {} MB should not exceed {}% of RAM ({} MB)",
                total,
                (RAM_SAFETY_FACTOR * 100.0) as usize,
                ram_mb
            );
        }
    }

    #[test]
    fn test_memory_multiplier_scaling() {
        let size_2x = detect_memory_size(2);
        let size_4x = detect_memory_size(4);
        let size_8x = detect_memory_size(8);

        assert!(size_2x >= MIN_BUFFER_MB);
        assert!(size_4x >= size_2x);
        assert!(size_8x >= size_4x);
    }
}
