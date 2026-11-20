use std::ops::Range;

const MAGIC_64: u32 = 0xfeed_facf;
const CPU_TYPE_X86_64: u32 = 0x0100_0007;
const HEADER_SIZE: usize = 32;
const LOAD_COMMAND_SEGMENT_64: u32 = 0x19;
const LOAD_COMMAND_UUID: u32 = 0x1b;
const SEGMENT_COMMAND_SIZE: usize = 72;
const SECTION_SIZE: usize = 80;
const NAME_SIZE: usize = 16;

pub struct MachO {
    bytes: Vec<u8>,
    pub uuid: [u8; 16],
    pub base: u64,
    segments: Vec<Segment>,
    sections: Vec<Section>,
}

struct Segment {
    addresses: Range<u64>,
    file_offset: u64,
    file_size: u64,
}

struct Section {
    segment: String,
    name: String,
    addresses: Range<u64>,
}

impl MachO {
    pub fn parse(bytes: Vec<u8>) -> Option<Self> {
        if read_u32(&bytes, 0)? != MAGIC_64 || read_u32(&bytes, 4)? != CPU_TYPE_X86_64 {
            return None;
        }
        let command_count = read_u32(&bytes, 16)?;
        let mut mach_o = MachO {
            bytes: Vec::new(),
            uuid: [0; 16],
            base: 0,
            segments: Vec::new(),
            sections: Vec::new(),
        };
        let mut position = HEADER_SIZE;
        for _ in 0..command_count {
            let command = bytes.get(position..)?;
            let kind = read_u32(command, 0)?;
            let size = read_u32(command, 4)? as usize;
            if size < 8 {
                return None;
            }
            match kind {
                LOAD_COMMAND_SEGMENT_64 => mach_o.add_segment(command.get(..size)?)?,
                LOAD_COMMAND_UUID => mach_o.uuid = command.get(8..24)?.try_into().ok()?,
                _ => {}
            }
            position += size;
        }
        mach_o.bytes = bytes;
        Some(mach_o)
    }

    fn add_segment(&mut self, command: &[u8]) -> Option<()> {
        let segment = read_name(command, 8)?;
        let address = read_u64(command, 24)?;
        let file_offset = read_u64(command, 40)?;
        if file_offset == 0 && segment == "__TEXT" {
            self.base = address;
        }
        self.segments.push(Segment {
            addresses: address..address.checked_add(read_u64(command, 32)?)?,
            file_offset,
            file_size: read_u64(command, 48)?,
        });
        for index in 0..read_u32(command, 64)? as usize {
            let section = command.get(SEGMENT_COMMAND_SIZE + index * SECTION_SIZE..)?;
            let address = read_u64(section, 32)?;
            self.sections.push(Section {
                segment: read_name(section, NAME_SIZE)?,
                name: read_name(section, 0)?,
                addresses: address..address.checked_add(read_u64(section, 40)?)?,
            });
        }
        Some(())
    }

    pub fn section(&self, segment: &str, name: &str) -> Option<Range<u64>> {
        self.sections
            .iter()
            .find(|section| section.segment == segment && section.name == name)
            .map(|section| section.addresses.clone())
    }

    pub fn bytes(&self, addresses: Range<u64>) -> Option<&[u8]> {
        let segment = self.segments.iter().find(|segment| {
            segment.addresses.contains(&addresses.start) && addresses.end <= segment.addresses.end
        })?;
        let start = addresses.start - segment.addresses.start;
        let end = addresses.end - segment.addresses.start;
        if end > segment.file_size {
            return None;
        }
        let file_start = usize::try_from(segment.file_offset + start).ok()?;
        let file_end = usize::try_from(segment.file_offset + end).ok()?;
        self.bytes.get(file_start..file_end)
    }

    pub fn u64_at(&self, address: u64) -> Option<u64> {
        read_u64(self.bytes(address..address.checked_add(8)?)?, 0)
    }
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

fn read_u64(bytes: &[u8], offset: usize) -> Option<u64> {
    Some(u64::from_le_bytes(
        bytes.get(offset..offset + 8)?.try_into().ok()?,
    ))
}

fn read_name(bytes: &[u8], offset: usize) -> Option<String> {
    let field = bytes.get(offset..offset + NAME_SIZE)?;
    let length = field
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(NAME_SIZE);
    Some(String::from_utf8_lossy(&field[..length]).into_owned())
}
