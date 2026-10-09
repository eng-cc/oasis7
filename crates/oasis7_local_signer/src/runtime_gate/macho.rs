use std::collections::BTreeSet;

use super::GateError;

const FAT_MAGIC: u32 = 0xcafebabe;
const FAT_CIGAM: u32 = 0xbebafeca;
const FAT_MAGIC_64: u32 = 0xcafebabf;
const FAT_CIGAM_64: u32 = 0xbfbafeca;
const MH_MAGIC_64: u32 = 0xfeedfacf;
const MH_CIGAM_64: u32 = 0xcffaedfe;
const CPU_TYPE_X86_64: u32 = 0x01000007;
const CPU_TYPE_ARM64: u32 = 0x0100000c;
const LC_LOAD_DYLIB: u32 = 0x0000000c;
const LC_LOAD_WEAK_DYLIB: u32 = 0x80000018;
const LC_REEXPORT_DYLIB: u32 = 0x8000001f;
const LC_LAZY_LOAD_DYLIB: u32 = 0x00000020;
const LC_LOAD_UPWARD_DYLIB: u32 = 0x80000023;
const LC_ID_DYLIB: u32 = 0x0000000d;
const LC_LOAD_DYLINKER: u32 = 0x0000000e;
const LC_RPATH: u32 = 0x8000001c;
const LC_DYLD_ENVIRONMENT: u32 = 0x00000027;

#[derive(Debug, PartialEq, Eq)]
struct SliceLoads {
    architecture: &'static str,
    dylibs: BTreeSet<String>,
    install_id: Option<String>,
    dylinker: Option<String>,
}

fn checked_slice(bytes: &[u8], start: usize, length: usize) -> Result<&[u8], GateError> {
    let end = start.checked_add(length).ok_or(GateError::MalformedMachO)?;
    bytes.get(start..end).ok_or(GateError::MalformedMachO)
}

fn u32_at(bytes: &[u8], offset: usize, little: bool) -> Result<u32, GateError> {
    let data: [u8; 4] = checked_slice(bytes, offset, 4)?
        .try_into()
        .map_err(|_| GateError::MalformedMachO)?;
    Ok(if little {
        u32::from_le_bytes(data)
    } else {
        u32::from_be_bytes(data)
    })
}

fn u64_at(bytes: &[u8], offset: usize, little: bool) -> Result<u64, GateError> {
    let data: [u8; 8] = checked_slice(bytes, offset, 8)?
        .try_into()
        .map_err(|_| GateError::MalformedMachO)?;
    Ok(if little {
        u64::from_le_bytes(data)
    } else {
        u64::from_be_bytes(data)
    })
}

fn load_command_name(
    command: &[u8],
    little: bool,
    minimum_size: usize,
) -> Result<String, GateError> {
    if command.len() < minimum_size {
        return Err(GateError::MalformedMachO);
    }
    let offset =
        usize::try_from(u32_at(command, 8, little)?).map_err(|_| GateError::MalformedMachO)?;
    if offset < minimum_size || offset >= command.len() {
        return Err(GateError::MalformedMachO);
    }
    let name_bytes = &command[offset..];
    let end = name_bytes
        .iter()
        .position(|byte| *byte == 0)
        .ok_or(GateError::MalformedMachO)?;
    let name = &name_bytes[..end];
    if name.is_empty() || !name.is_ascii() {
        return Err(GateError::MalformedMachO);
    }
    String::from_utf8(name.to_vec()).map_err(|_| GateError::MalformedMachO)
}

fn parse_slice(bytes: &[u8]) -> Result<SliceLoads, GateError> {
    if bytes.len() < 32 {
        return Err(GateError::MalformedMachO);
    }
    let magic_be = u32::from_be_bytes(
        bytes[..4]
            .try_into()
            .map_err(|_| GateError::MalformedMachO)?,
    );
    let little = match magic_be {
        MH_MAGIC_64 => false,
        MH_CIGAM_64 => true,
        _ => return Err(GateError::MalformedMachO),
    };
    let cpu_type = u32_at(bytes, 4, little)?;
    let architecture = match cpu_type {
        CPU_TYPE_X86_64 => "x86_64",
        CPU_TYPE_ARM64 => "arm64",
        _ => return Err(GateError::MalformedMachO),
    };
    let command_count =
        usize::try_from(u32_at(bytes, 16, little)?).map_err(|_| GateError::MalformedMachO)?;
    let command_bytes =
        usize::try_from(u32_at(bytes, 20, little)?).map_err(|_| GateError::MalformedMachO)?;
    if command_count > 4096 || command_bytes > bytes.len().saturating_sub(32) {
        return Err(GateError::MalformedMachO);
    }
    let command_end = 32usize
        .checked_add(command_bytes)
        .ok_or(GateError::MalformedMachO)?;
    let mut cursor = 32usize;
    let mut dylibs = BTreeSet::new();
    let mut install_id = None;
    let mut dylinker = None;

    for _ in 0..command_count {
        let command = u32_at(bytes, cursor, little)?;
        let size = usize::try_from(u32_at(bytes, cursor + 4, little)?)
            .map_err(|_| GateError::MalformedMachO)?;
        if size < 8 || cursor.checked_add(size).is_none_or(|end| end > command_end) {
            return Err(GateError::MalformedMachO);
        }
        let record = checked_slice(bytes, cursor, size)?;
        match command {
            LC_LOAD_DYLIB | LC_LOAD_WEAK_DYLIB | LC_REEXPORT_DYLIB | LC_LAZY_LOAD_DYLIB
            | LC_LOAD_UPWARD_DYLIB => {
                if !dylibs.insert(load_command_name(record, little, 24)?) {
                    return Err(GateError::MalformedMachO);
                }
            }
            LC_ID_DYLIB => {
                if install_id
                    .replace(load_command_name(record, little, 24)?)
                    .is_some()
                {
                    return Err(GateError::MalformedMachO);
                }
            }
            LC_LOAD_DYLINKER => {
                if dylinker
                    .replace(load_command_name(record, little, 12)?)
                    .is_some()
                {
                    return Err(GateError::MalformedMachO);
                }
            }
            LC_RPATH | LC_DYLD_ENVIRONMENT => return Err(GateError::UnsafeMachOPath),
            _ => {}
        }
        cursor += size;
    }
    if cursor != command_end {
        return Err(GateError::MalformedMachO);
    }
    Ok(SliceLoads {
        architecture,
        dylibs,
        install_id,
        dylinker,
    })
}

fn parse_image(bytes: &[u8]) -> Result<Vec<SliceLoads>, GateError> {
    if bytes.len() < 8 {
        return Err(GateError::MalformedMachO);
    }
    let magic_be = u32::from_be_bytes(
        bytes[..4]
            .try_into()
            .map_err(|_| GateError::MalformedMachO)?,
    );
    if !matches!(
        magic_be,
        FAT_MAGIC | FAT_CIGAM | FAT_MAGIC_64 | FAT_CIGAM_64
    ) {
        return Ok(vec![parse_slice(bytes)?]);
    }
    let little = matches!(magic_be, FAT_CIGAM | FAT_CIGAM_64);
    let wide = matches!(magic_be, FAT_MAGIC_64 | FAT_CIGAM_64);
    let count =
        usize::try_from(u32_at(bytes, 4, little)?).map_err(|_| GateError::MalformedMachO)?;
    if count != 2 {
        return Err(GateError::UnexpectedMachOArchitectures);
    }
    let record_size = if wide { 32 } else { 20 };
    let records_end = 8usize
        .checked_add(count * record_size)
        .ok_or(GateError::MalformedMachO)?;
    if records_end > bytes.len() {
        return Err(GateError::MalformedMachO);
    }
    let mut slices = Vec::with_capacity(count);
    let mut ranges = Vec::with_capacity(count);
    for index in 0..count {
        let base = 8 + index * record_size;
        let expected_cpu = u32_at(bytes, base, little)?;
        let (offset, length) = if wide {
            (
                usize::try_from(u64_at(bytes, base + 8, little)?)
                    .map_err(|_| GateError::MalformedMachO)?,
                usize::try_from(u64_at(bytes, base + 16, little)?)
                    .map_err(|_| GateError::MalformedMachO)?,
            )
        } else {
            (
                usize::try_from(u32_at(bytes, base + 8, little)?)
                    .map_err(|_| GateError::MalformedMachO)?,
                usize::try_from(u32_at(bytes, base + 12, little)?)
                    .map_err(|_| GateError::MalformedMachO)?,
            )
        };
        if offset < records_end || length == 0 {
            return Err(GateError::MalformedMachO);
        }
        let end = offset
            .checked_add(length)
            .ok_or(GateError::MalformedMachO)?;
        if ranges
            .iter()
            .any(|(start, prior_end)| offset < *prior_end && *start < end)
        {
            return Err(GateError::MalformedMachO);
        }
        ranges.push((offset, end));
        let slice = parse_slice(checked_slice(bytes, offset, length)?)?;
        let slice_cpu = match slice.architecture {
            "x86_64" => CPU_TYPE_X86_64,
            "arm64" => CPU_TYPE_ARM64,
            _ => return Err(GateError::UnexpectedMachOArchitectures),
        };
        if expected_cpu != slice_cpu {
            return Err(GateError::MalformedMachO);
        }
        slices.push(slice);
    }
    let architectures = slices
        .iter()
        .map(|slice| slice.architecture)
        .collect::<BTreeSet<_>>();
    if architectures != BTreeSet::from(["arm64", "x86_64"]) {
        return Err(GateError::UnexpectedMachOArchitectures);
    }
    Ok(slices)
}

fn expected_for_component(component: &str) -> Result<BTreeSet<&'static str>, GateError> {
    const SYSTEM: &str = "/usr/lib/libSystem.B.dylib";
    let names: &[&str] = match component {
        "python3.9" => &["@executable_path/../Python3", SYSTEM],
        "Python3" => &[
            "/System/Library/Frameworks/CoreFoundation.framework/Versions/A/CoreFoundation",
            SYSTEM,
        ],
        "_blake2.cpython-39-darwin.so"
        | "_heapq.cpython-39-darwin.so"
        | "_json.cpython-39-darwin.so"
        | "_posixsubprocess.cpython-39-darwin.so"
        | "_sha3.cpython-39-darwin.so"
        | "_struct.cpython-39-darwin.so"
        | "fcntl.cpython-39-darwin.so"
        | "grp.cpython-39-darwin.so"
        | "math.cpython-39-darwin.so"
        | "select.cpython-39-darwin.so" => &[SYSTEM],
        "_bz2.cpython-39-darwin.so" => &["/usr/lib/libbz2.1.0.dylib", SYSTEM],
        "_ctypes.cpython-39-darwin.so" => &["/usr/lib/libffi.dylib", SYSTEM],
        "_hashlib.cpython-39-darwin.so" => &[
            "/usr/lib/libcrypto.44.dylib",
            "/usr/lib/libssl.46.dylib",
            "/System/Library/PrivateFrameworks/TrustEvaluationAgent.framework/Versions/A/TrustEvaluationAgent",
            SYSTEM,
        ],
        "_lzma.cpython-39-darwin.so" => &["/usr/lib/liblzma.5.dylib", SYSTEM],
        "zlib.cpython-39-darwin.so" => &["/usr/lib/libz.1.dylib", SYSTEM],
        _ => return Err(GateError::UnexpectedDependencyComponent),
    };
    Ok(names.iter().copied().collect())
}

pub(super) fn verify_component(bytes: &[u8], component: &str) -> Result<(), GateError> {
    let slices = parse_image(bytes)?;
    let architectures = slices
        .iter()
        .map(|slice| slice.architecture)
        .collect::<BTreeSet<_>>();
    if architectures != BTreeSet::from(["arm64", "x86_64"]) {
        return Err(GateError::UnexpectedMachOArchitectures);
    }
    let expected = expected_for_component(component)?;
    for slice in slices {
        if slice
            .dylibs
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>()
            != expected
        {
            return Err(GateError::UnexpectedMachODependency);
        }
        let expected_id =
            (component == "Python3").then_some("@rpath/Python3.framework/Versions/3.9/Python3");
        if slice.install_id.as_deref() != expected_id {
            return Err(GateError::UnexpectedMachODependency);
        }
        let expected_dylinker = (component == "python3.9").then_some("/usr/lib/dyld");
        if slice.dylinker.as_deref() != expected_dylinker {
            return Err(GateError::UnexpectedMachODependency);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn thin_macho(loads: &[&str], dylinker: Option<&str>, cpu_type: u32) -> Vec<u8> {
        let mut commands = Vec::new();
        for name in loads {
            let mut command = vec![0; 24];
            command[..4].copy_from_slice(&LC_LOAD_DYLIB.to_le_bytes());
            let name_offset = 24u32;
            command[8..12].copy_from_slice(&name_offset.to_le_bytes());
            command.extend_from_slice(name.as_bytes());
            command.push(0);
            while command.len() % 8 != 0 {
                command.push(0);
            }
            let size = u32::try_from(command.len()).expect("test command size");
            command[4..8].copy_from_slice(&size.to_le_bytes());
            commands.extend_from_slice(&command);
        }
        if let Some(name) = dylinker {
            let mut command = vec![0; 12];
            command[..4].copy_from_slice(&LC_LOAD_DYLINKER.to_le_bytes());
            command[8..12].copy_from_slice(&12u32.to_le_bytes());
            command.extend_from_slice(name.as_bytes());
            command.push(0);
            while command.len() % 4 != 0 {
                command.push(0);
            }
            let size = u32::try_from(command.len()).expect("test command size");
            command[4..8].copy_from_slice(&size.to_le_bytes());
            commands.extend_from_slice(&command);
        }
        let ncmds = u32::try_from(loads.len() + usize::from(dylinker.is_some()))
            .expect("test command count");
        let sizeofcmds = u32::try_from(commands.len()).expect("test command bytes");
        let mut image = vec![0; 32];
        image[..4].copy_from_slice(&MH_MAGIC_64.to_le_bytes());
        image[4..8].copy_from_slice(&cpu_type.to_le_bytes());
        image[12..16].copy_from_slice(&2u32.to_le_bytes());
        image[16..20].copy_from_slice(&ncmds.to_le_bytes());
        image[20..24].copy_from_slice(&sizeofcmds.to_le_bytes());
        image.extend_from_slice(&commands);
        image
    }

    fn universal_macho(loads: &[&str], dylinker: Option<&str>) -> Vec<u8> {
        let arm = thin_macho(loads, dylinker, CPU_TYPE_ARM64);
        let x86 = thin_macho(loads, dylinker, CPU_TYPE_X86_64);
        let records_end = 8 + 2 * 32;
        let arm_offset = records_end;
        let x86_offset = arm_offset + arm.len();
        let mut image = Vec::with_capacity(x86_offset + x86.len());
        image.extend_from_slice(&FAT_MAGIC_64.to_be_bytes());
        image.extend_from_slice(&2u32.to_be_bytes());
        for (cpu, offset, slice) in [
            (CPU_TYPE_ARM64, arm_offset, &arm),
            (CPU_TYPE_X86_64, x86_offset, &x86),
        ] {
            image.extend_from_slice(&cpu.to_be_bytes());
            image.extend_from_slice(&0u32.to_be_bytes());
            image.extend_from_slice(&(offset as u64).to_be_bytes());
            image.extend_from_slice(&(slice.len() as u64).to_be_bytes());
            image.extend_from_slice(&0u32.to_be_bytes());
            image.extend_from_slice(&0u32.to_be_bytes());
        }
        image.extend_from_slice(&arm);
        image.extend_from_slice(&x86);
        image
    }

    #[test]
    fn rejects_unlisted_dylib_and_unsafe_rpath() {
        let extra = universal_macho(
            &["/usr/lib/libSystem.B.dylib", "/tmp/attacker.dylib"],
            Some("/usr/lib/dyld"),
        );
        assert_eq!(
            verify_component(&extra, "_posixsubprocess.cpython-39-darwin.so"),
            Err(GateError::UnexpectedMachODependency)
        );

        let mut rpath = thin_macho(
            &["/usr/lib/libSystem.B.dylib"],
            Some("/usr/lib/dyld"),
            CPU_TYPE_ARM64,
        );
        let mut command = vec![0; 20];
        command[..4].copy_from_slice(&LC_RPATH.to_le_bytes());
        command[4..8].copy_from_slice(&16u32.to_le_bytes());
        command[8..12].copy_from_slice(&12u32.to_le_bytes());
        command[12..17].copy_from_slice(b"/tmp\0");
        let ncmds = u32::from_le_bytes(rpath[16..20].try_into().expect("header"));
        let size = u32::from_le_bytes(rpath[20..24].try_into().expect("header"));
        rpath.extend_from_slice(&command);
        rpath[16..20].copy_from_slice(&(ncmds + 1).to_le_bytes());
        rpath[20..24].copy_from_slice(&(size + 20).to_le_bytes());
        assert_eq!(parse_image(&rpath), Err(GateError::UnsafeMachOPath));
    }

    #[test]
    fn rejects_truncated_or_non_universal_runtime_images() {
        assert_eq!(parse_image(&[0; 12]), Err(GateError::MalformedMachO));
        let arm_only = thin_macho(
            &["/usr/lib/libSystem.B.dylib"],
            Some("/usr/lib/dyld"),
            CPU_TYPE_ARM64,
        );
        assert_eq!(
            verify_component(&arm_only, "python3.9"),
            Err(GateError::UnexpectedMachOArchitectures)
        );
    }
}
