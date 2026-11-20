use super::mach_o::MachO;
use iced_x86::{
    Decoder, DecoderOptions, Instruction, InstructionInfoFactory, Mnemonic, OpAccess, OpKind,
    Register,
};
use std::collections::BTreeMap;
use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

const DEVICE_GL: &str = "N3RBX8Graphics8DeviceGLE";
const DEBUG_DEVICE_VR: &str = "N3RBX8Graphics13DebugDeviceVRE";
const FRAMEBUFFER_GL: &str = "N3RBX8Graphics13FramebufferGLE";
const EMULATOR_FLAG: &str = "DebugEnableVREmulator";

const GL_SLOT_BEGIN_FRAME: usize = 13;
const GL_SLOT_PRESENT: usize = 14;
const GL_SLOT_MAIN_FRAMEBUFFER: usize = 15;
const GL_SLOTS_SEARCHED_FOR_GET_VR: Range<usize> = 16..40;

const VR_SLOT_GET_STATE: usize = 5;
const VR_SLOT_EYE_FRAMEBUFFER: usize = 9;
const VR_SLOTS_MAX: usize = 64;
const BEGIN_FRAME_AFTER_CREATE_RESOURCES: usize = 1;
const END_FRAME_AFTER_CREATE_RESOURCES: usize = 2;
const PRESENT_WINDOW_AFTER_CREATE_RESOURCES: usize = 5;
const DISPLAY_RATE_AFTER_CREATE_RESOURCES: usize = 11;
const SET_DISPLAY_RATE_AFTER_CREATE_RESOURCES: usize = 12;

const STATE_SIZE: u64 = 0x138;
const FRAMEBUFFER_GL_NAME: i64 = 0x8c;
const MEMBER_OFFSET_MAX: u64 = 0x10000;
const POINTER_SIZE: u64 = 8;
const DISPLACEMENT_SIZE: u64 = 4;
const LONGEST_INSTRUCTION: u64 = 15;
const PROLOGUE_SEARCH: u64 = 0x400;

const READS: [OpAccess; 4] = [
    OpAccess::Read,
    OpAccess::CondRead,
    OpAccess::ReadWrite,
    OpAccess::ReadCondWrite,
];
const WRITES: [OpAccess; 4] = [
    OpAccess::Write,
    OpAccess::CondWrite,
    OpAccess::ReadWrite,
    OpAccess::ReadCondWrite,
];
const PRESERVED_ACROSS_CALLS: [Register; 6] = [
    Register::RBX,
    Register::RBP,
    Register::R12,
    Register::R13,
    Register::R14,
    Register::R15,
];

static FOUND_LAYOUTS: Mutex<BTreeMap<PathBuf, FoundLayout>> = Mutex::new(BTreeMap::new());

struct FoundLayout {
    modified: SystemTime,
    size: u64,
    layout: Result<VrLayout, String>,
}

#[derive(Clone)]
pub struct VrLayout {
    uuid: [u8; 16],
    gl_vtable: u64,
    debug_vtable: u64,
    debug_constructor: u64,
    emulator_flag: Option<u64>,
    debug_methods: usize,
    gl_get_vr_slot: usize,
    device_offset: u64,
    enabled_offset: u64,
    state_offset: u64,
    create_resources_slot: usize,
    mirror: bool,
}

impl VrLayout {
    pub fn environment_value(&self) -> String {
        let create_resources = self.create_resources_slot as u64;
        let mut fields = vec![
            ("gl_vtable", self.gl_vtable),
            ("debug_vtable", self.debug_vtable),
            ("debug_constructor", self.debug_constructor),
            ("debug_methods", self.debug_methods as u64),
            ("gl_begin_frame_slot", GL_SLOT_BEGIN_FRAME as u64),
            ("gl_main_framebuffer_slot", GL_SLOT_MAIN_FRAMEBUFFER as u64),
            ("gl_get_vr_slot", self.gl_get_vr_slot as u64),
            ("gl_set_vr_slot", self.gl_get_vr_slot as u64 + 1),
            ("device_offset", self.device_offset),
            ("enabled_offset", self.enabled_offset),
            ("state_offset", self.state_offset),
            ("create_resources_slot", create_resources),
            (
                "begin_frame_slot",
                create_resources + BEGIN_FRAME_AFTER_CREATE_RESOURCES as u64,
            ),
            (
                "end_frame_slot",
                create_resources + END_FRAME_AFTER_CREATE_RESOURCES as u64,
            ),
            (
                "present_window_slot",
                create_resources + PRESENT_WINDOW_AFTER_CREATE_RESOURCES as u64,
            ),
            (
                "display_rate_slot",
                create_resources + DISPLAY_RATE_AFTER_CREATE_RESOURCES as u64,
            ),
            (
                "set_display_rate_slot",
                create_resources + SET_DISPLAY_RATE_AFTER_CREATE_RESOURCES as u64,
            ),
            ("mirror", u64::from(self.mirror)),
        ];
        if let Some(emulator_flag) = self.emulator_flag {
            fields.push(("emulator_flag", emulator_flag));
        }
        let uuid: String = self.uuid.iter().map(|byte| format!("{byte:02x}")).collect();
        std::iter::once(format!("uuid={uuid}"))
            .chain(
                fields
                    .into_iter()
                    .map(|(name, value)| format!("{name}={value:x}")),
            )
            .collect::<Vec<_>>()
            .join(",")
    }
}

pub fn of(executable: &Path) -> Result<VrLayout, String> {
    let unreadable =
        |error: std::io::Error| format!("Couldn't read {}: {error}", executable.display());
    let metadata = executable.metadata().map_err(unreadable)?;
    let modified = metadata.modified().map_err(unreadable)?;
    let mut found = FOUND_LAYOUTS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    if let Some(earlier) = found.get(executable)
        && earlier.modified == modified
        && earlier.size == metadata.len()
    {
        return earlier.layout.clone();
    }

    let layout = fs::read(executable)
        .map_err(unreadable)
        .and_then(|bytes| Binary::new(bytes)?.layout());
    found.insert(
        executable.to_owned(),
        FoundLayout {
            modified,
            size: metadata.len(),
            layout: layout.clone(),
        },
    );
    layout
}

struct Binary {
    mach_o: MachO,
    text: Range<u64>,
    text_const: Range<u64>,
    cstring: Range<u64>,
    data_const: Range<u64>,
    data_const_words: Vec<u64>,
    data: Option<Range<u64>>,
}

impl Binary {
    fn new(bytes: Vec<u8>) -> Result<Self, String> {
        let mach_o = MachO::parse(bytes).ok_or("Roblox isn't a 64-bit Intel Mac program")?;
        let section = |segment: &str, name: &str| {
            mach_o
                .section(segment, name)
                .ok_or(format!("Roblox has no {segment},{name} section"))
        };
        let text = section("__TEXT", "__text")?;
        let text_const = section("__TEXT", "__const")?;
        let cstring = section("__TEXT", "__cstring")?;
        let data_const = section("__DATA", "__const")?;
        let data = mach_o.section("__DATA", "__data");
        let data_const_words = mach_o
            .bytes(data_const.clone())
            .map(words)
            .unwrap_or_default();
        Ok(Self {
            mach_o,
            text,
            text_const,
            cstring,
            data_const,
            data_const_words,
            data,
        })
    }

    fn layout(&self) -> Result<VrLayout, String> {
        let (Some(gl_vtable), Some(debug_vtable)) =
            (self.vtable(DEVICE_GL), self.vtable(DEBUG_DEVICE_VR))
        else {
            return Err("Roblox's graphics or VR device class wasn't found".to_owned());
        };
        let state_offset = self
            .method(debug_vtable, VR_SLOT_GET_STATE)
            .and_then(|get_state| self.read_state_offset(get_state))
            .ok_or("DebugDeviceVR::getState doesn't copy the VR state as expected")?;
        let (gl_get_vr_slot, device_offset, enabled_offset) = self
            .find_gl_vr_accessors(gl_vtable)
            .ok_or("DeviceGL's getVR and setVR weren't found")?;
        let create_resources_slot = self
            .find_create_resources_slot(gl_vtable, device_offset)
            .ok_or("DeviceGL doesn't call the VR device's frame methods as expected")?;
        let debug_methods = self.count_methods(debug_vtable);
        if create_resources_slot + SET_DISPLAY_RATE_AFTER_CREATE_RESOURCES >= debug_methods
            || debug_methods >= VR_SLOTS_MAX
        {
            return Err("DebugDeviceVR's method table looks different".to_owned());
        }
        let debug_constructor = self
            .find_debug_constructor(debug_vtable)
            .ok_or("DebugDeviceVR's constructor wasn't found")?;

        let base = self.mach_o.base;
        Ok(VrLayout {
            uuid: self.mach_o.uuid,
            gl_vtable: gl_vtable - base,
            debug_vtable: debug_vtable - base,
            debug_constructor: debug_constructor - base,
            emulator_flag: self.emulator_flag().map(|flag| flag - base),
            debug_methods,
            gl_get_vr_slot,
            device_offset,
            enabled_offset,
            state_offset,
            create_resources_slot,
            mirror: self.supports_mirror(gl_vtable),
        })
    }

    fn names(&self, section: &Range<u64>, name: &str) -> Vec<u64> {
        let Some(bytes) = self.mach_o.bytes(section.clone()) else {
            return Vec::new();
        };
        let wanted: Vec<u8> = name.bytes().chain([0]).collect();
        bytes
            .windows(wanted.len())
            .enumerate()
            .filter(|&(index, window)| {
                window == wanted
                    && !index
                        .checked_sub(1)
                        .is_some_and(|previous| is_identifier_byte(bytes[previous]))
            })
            .map(|(index, _)| section.start + index as u64)
            .collect()
    }

    fn vtable(&self, mangled: &str) -> Option<u64> {
        [&self.text_const, &self.cstring]
            .into_iter()
            .flat_map(|section| self.names(section, mangled))
            .find_map(|name| self.vtable_named(name))
    }

    fn vtable_named(&self, name: u64) -> Option<u64> {
        let words = &self.data_const_words;
        let address_of = |index: usize| self.data_const.start + index as u64 * POINTER_SIZE;
        (1..words.len())
            .filter(|&index| words[index] == name)
            .find_map(|index| {
                let typeinfo = address_of(index - 1);
                (1..words.len())
                    .find(|&slot| words[slot] == typeinfo && words[slot - 1] == 0)
                    .map(|slot| address_of(slot + 1))
            })
    }

    fn method(&self, vtable: u64, slot: usize) -> Option<u64> {
        self.mach_o
            .u64_at(vtable + slot as u64 * POINTER_SIZE)
            .filter(|method| self.text.contains(method))
    }

    fn count_methods(&self, vtable: u64) -> usize {
        (0..VR_SLOTS_MAX)
            .take_while(|&slot| {
                self.data_const
                    .contains(&(vtable + slot as u64 * POINTER_SIZE))
                    && self.method(vtable, slot).is_some()
            })
            .count()
    }

    fn instructions(&self, address: u64, limit: usize) -> impl Iterator<Item = Instruction> + '_ {
        let code = if self.text.contains(&address) {
            self.mach_o
                .bytes(address..self.text.end)
                .unwrap_or_default()
        } else {
            &[]
        };
        let mut decoder = Decoder::with_ip(64, code, address, DecoderOptions::NONE);
        let mut returned = false;
        std::iter::from_fn(move || {
            if returned || !decoder.can_decode() {
                return None;
            }
            let instruction = decoder.decode();
            returned = instruction.mnemonic() == Mnemonic::Ret;
            (!instruction.is_invalid()).then_some(instruction)
        })
        .take(limit)
    }

    fn function(&self, address: u64, limit: usize) -> Vec<Instruction> {
        self.instructions(address, limit).collect()
    }

    fn read_state_offset(&self, get_state: u64) -> Option<u64> {
        let mut offset = None;
        let mut copies_whole_state = false;
        for instruction in self.instructions(get_state, 16) {
            match instruction.mnemonic() {
                Mnemonic::Add if operand_register(&instruction, 0) == Some(Register::RSI) => {
                    offset = operand_immediate(&instruction, 1)
                        .filter(|&offset| offset > 0 && offset < MEMBER_OFFSET_MAX);
                }
                Mnemonic::Mov
                    if operand_register(&instruction, 0) == Some(Register::EDX)
                        && operand_immediate(&instruction, 1) == Some(STATE_SIZE) =>
                {
                    copies_whole_state = true;
                }
                Mnemonic::Call => return offset.filter(|_| copies_whole_state),
                _ => {}
            }
        }
        None
    }

    fn find_gl_vr_accessors(&self, gl_vtable: u64) -> Option<(usize, u64, u64)> {
        GL_SLOTS_SEARCHED_FOR_GET_VR.into_iter().find_map(|slot| {
            let (device, enabled) = self.read_get_vr(self.method(gl_vtable, slot)?)?;
            let stored = self.read_set_vr(self.method(gl_vtable, slot + 1)?)?;
            (stored == enabled).then_some((slot, device, enabled))
        })
    }

    fn read_get_vr(&self, get_vr: u64) -> Option<(u64, u64)> {
        let body = self.function(get_vr, 12);
        let mut held = None;
        let mut device = None;
        let mut enabled = None;
        let mut selects_device = false;
        for instruction in &body {
            if let Some((destination, Register::RDI, offset)) = load(instruction)
                && destination.size() == 8
            {
                held = Some(destination);
                device = member_offset(offset);
            } else if instruction.mnemonic() == Mnemonic::Cmp
                && let Some((Register::RDI, offset)) = operand_member(instruction, 0)
                && instruction.memory_size().size() == 1
                && operand_immediate(instruction, 1) == Some(0)
            {
                enabled = member_offset(offset);
            } else if instruction.mnemonic() == Mnemonic::Cmovne
                && held.is_some()
                && operand_register(instruction, 0) == Some(Register::RAX)
                && operand_register(instruction, 1) == held
            {
                selects_device = true;
            }
        }
        (returns(&body) && selects_device).then_some((device?, enabled?))
    }

    fn read_set_vr(&self, set_vr: u64) -> Option<u64> {
        let body = self.function(set_vr, 6);
        let stored = body.iter().find_map(|instruction| {
            if instruction.mnemonic() == Mnemonic::Mov
                && operand_register(instruction, 1) == Some(Register::SIL)
                && let Some((Register::RDI, offset)) = operand_member(instruction, 0)
            {
                member_offset(offset)
            } else {
                None
            }
        });
        stored.filter(|_| returns(&body))
    }

    fn find_create_resources_slot(&self, gl_vtable: u64, device_offset: u64) -> Option<usize> {
        let begin_frame_calls =
            self.device_calls(self.method(gl_vtable, GL_SLOT_BEGIN_FRAME)?, device_offset);
        let present_calls =
            self.device_calls(self.method(gl_vtable, GL_SLOT_PRESENT)?, device_offset);
        let create_resources = begin_frame_calls
            .first()?
            .checked_sub(BEGIN_FRAME_AFTER_CREATE_RESOURCES)?;
        let present_window_then_end_frame = [
            create_resources + PRESENT_WINDOW_AFTER_CREATE_RESOURCES,
            create_resources + END_FRAME_AFTER_CREATE_RESOURCES,
        ];
        (create_resources > VR_SLOT_EYE_FRAMEBUFFER
            && present_calls.starts_with(&present_window_then_end_frame))
        .then_some(create_resources)
    }

    fn device_calls(&self, function: u64, device_offset: u64) -> Vec<usize> {
        let mut this = register_bit(Register::RDI);
        let mut device = 0;
        let mut table = 0;
        let mut slots = Vec::new();
        for instruction in self.instructions(function, 1024) {
            let loaded = load(&instruction).filter(|(destination, _, _)| destination.size() == 8);
            let loads_device = loaded
                .filter(|&(_, base, offset)| {
                    this & register_bit(base) != 0 && member_offset(offset) == Some(device_offset)
                })
                .map(|(destination, _, _)| destination);
            let loads_table = loaded
                .filter(|&(_, base, offset)| device & register_bit(base) != 0 && offset == 0)
                .map(|(destination, _, _)| destination);
            if instruction.mnemonic() == Mnemonic::Call
                && let Some((base, offset)) = operand_member(&instruction, 0)
                && table & register_bit(base) != 0
                && let Some(slot) = slot_at(offset)
            {
                slots.push(slot);
            }
            this = still_held(&instruction, this);
            device = still_held(&instruction, device);
            table = still_held(&instruction, table);
            device |= loads_device.map_or(0, register_bit);
            table |= loads_table.map_or(0, register_bit);
        }
        slots
    }

    fn find_debug_constructor(&self, vtable: u64) -> Option<u64> {
        let text = self.mach_o.bytes(self.text.clone())?;
        text.windows(DISPLACEMENT_SIZE as usize)
            .enumerate()
            .filter(|(index, window)| {
                let relative = i32::from_le_bytes([window[0], window[1], window[2], window[3]]);
                (self.text.start + *index as u64 + DISPLACEMENT_SIZE)
                    .wrapping_add_signed(i64::from(relative))
                    == vtable
            })
            .find_map(|(index, _)| self.constructor_storing(self.text.start + index as u64, vtable))
    }

    fn constructor_storing(&self, displacement: u64, vtable: u64) -> Option<u64> {
        let width_height_and_device =
            register_bit(Register::RSI) | register_bit(Register::RDX) | register_bit(Register::RCX);
        (2..=LONGEST_INSTRUCTION - DISPLACEMENT_SIZE).find_map(|before| {
            let reference = self
                .instructions(displacement.checked_sub(before)?, 1)
                .next()?;
            let stores_vtable = reference.next_ip() == displacement + DISPLACEMENT_SIZE
                && reference.mnemonic() == Mnemonic::Lea
                && reference.is_ip_rel_memory_operand()
                && reference.ip_rel_memory_address() == vtable
                && self.stores_into_object(&reference);
            if !stores_vtable {
                return None;
            }
            let (function, arguments) = self.function_reading_arguments(reference.ip())?;
            (arguments & width_height_and_device == width_height_and_device).then_some(function)
        })
    }

    fn stores_into_object(&self, reference: &Instruction) -> bool {
        self.instructions(reference.next_ip(), 1)
            .next()
            .is_some_and(|store| {
                store.mnemonic() == Mnemonic::Mov
                    && matches!(operand_member(&store, 0), Some((_, 0)))
                    && operand_register(&store, 1) == operand_register(reference, 0)
            })
    }

    fn function_reading_arguments(&self, address: u64) -> Option<(u64, u32)> {
        (1..=PROLOGUE_SEARCH).find_map(|distance| {
            let start = address.checked_sub(distance)?;
            if !self.sets_up_frame(start) {
                return None;
            }
            let mut arguments = 0;
            let mut written = 0;
            for instruction in self.instructions(start, PROLOGUE_SEARCH as usize) {
                if instruction.ip() >= address {
                    return (instruction.ip() == address).then_some((start, arguments));
                }
                arguments |= accessed_registers(&instruction, &READS) & !written;
                written |= accessed_registers(&instruction, &WRITES);
            }
            None
        })
    }

    fn sets_up_frame(&self, address: u64) -> bool {
        let prologue = self.function(address, 2);
        matches!(prologue.as_slice(), [push, frame]
            if push.mnemonic() == Mnemonic::Push
                && operand_register(push, 0) == Some(Register::RBP)
                && frame.mnemonic() == Mnemonic::Mov
                && is_frame(frame))
    }

    fn supports_mirror(&self, gl_vtable: u64) -> bool {
        let framebuffer_destructor = self
            .vtable(FRAMEBUFFER_GL)
            .and_then(|vtable| self.method(vtable, 0));
        self.method(gl_vtable, GL_SLOT_MAIN_FRAMEBUFFER)
            .is_some_and(|main_framebuffer| self.is_member_getter(main_framebuffer))
            && framebuffer_destructor
                .is_some_and(|destructor| self.releases_gl_framebuffer(destructor))
    }

    fn is_member_getter(&self, function: u64) -> bool {
        let body = self.function(function, 8);
        let is_getter_load = |instruction: &Instruction| {
            matches!(load(instruction), Some((Register::RAX, Register::RDI, _)))
        };
        returns(&body)
            && body
                .iter()
                .filter(|instruction| is_getter_load(instruction))
                .count()
                == 1
            && body.iter().all(|instruction| {
                is_getter_load(instruction)
                    || is_frame(instruction)
                    || instruction.mnemonic() == Mnemonic::Ret
            })
    }

    fn releases_gl_framebuffer(&self, destructor: u64) -> bool {
        self.instructions(self.follow_tail_call(destructor), 64)
            .any(|instruction| {
                matches!(load(&instruction), Some((destination, _, FRAMEBUFFER_GL_NAME)) if destination.size() == 4)
            })
    }

    fn follow_tail_call(&self, function: u64) -> u64 {
        self.instructions(function, 4)
            .find(|instruction| !is_frame(instruction))
            .filter(|instruction| {
                instruction.mnemonic() == Mnemonic::Jmp
                    && instruction.op_kind(0) == OpKind::NearBranch64
            })
            .map_or(function, |jump| jump.near_branch_target())
    }

    fn emulator_flag(&self) -> Option<u64> {
        let data = self.data.as_ref()?;
        let words = words(self.mach_o.bytes(data.clone())?);
        let name_length = EMULATOR_FLAG.len() as u64;
        self.names(&self.cstring, EMULATOR_FLAG)
            .into_iter()
            .find_map(|name| {
                words
                    .windows(3)
                    .position(|flag| flag[1] == name && flag[2] == name_length)
            })
            .map(|index| data.start + index as u64 * POINTER_SIZE)
    }
}

fn words(bytes: &[u8]) -> Vec<u64> {
    let (words, _) = bytes.as_chunks();
    words.iter().map(|&word| u64::from_le_bytes(word)).collect()
}

fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn returns(body: &[Instruction]) -> bool {
    body.last()
        .is_some_and(|instruction| instruction.mnemonic() == Mnemonic::Ret)
}

fn member_offset(offset: i64) -> Option<u64> {
    u64::try_from(offset)
        .ok()
        .filter(|&offset| offset < MEMBER_OFFSET_MAX)
}

fn slot_at(offset: i64) -> Option<usize> {
    (offset >= 0 && offset % POINTER_SIZE as i64 == 0)
        .then(|| (offset / POINTER_SIZE as i64) as usize)
}

fn operand_register(instruction: &Instruction, operand: u32) -> Option<Register> {
    (operand < instruction.op_count() && instruction.op_kind(operand) == OpKind::Register)
        .then(|| instruction.op_register(operand))
}

fn operand_member(instruction: &Instruction, operand: u32) -> Option<(Register, i64)> {
    let base = instruction.memory_base();
    (operand < instruction.op_count()
        && instruction.op_kind(operand) == OpKind::Memory
        && instruction.memory_index() == Register::None
        && !matches!(base, Register::None | Register::RIP))
    .then(|| (base, instruction.memory_displacement64() as i64))
}

fn operand_immediate(instruction: &Instruction, operand: u32) -> Option<u64> {
    let is_immediate = operand < instruction.op_count()
        && matches!(
            instruction.op_kind(operand),
            OpKind::Immediate8
                | OpKind::Immediate16
                | OpKind::Immediate32
                | OpKind::Immediate64
                | OpKind::Immediate8to16
                | OpKind::Immediate8to32
                | OpKind::Immediate8to64
                | OpKind::Immediate32to64
        );
    is_immediate.then(|| instruction.immediate(operand))
}

fn load(instruction: &Instruction) -> Option<(Register, Register, i64)> {
    if instruction.mnemonic() != Mnemonic::Mov {
        return None;
    }
    let destination = operand_register(instruction, 0)?;
    let (base, offset) = operand_member(instruction, 1)?;
    Some((destination, base, offset))
}

fn is_frame(instruction: &Instruction) -> bool {
    match instruction.mnemonic() {
        Mnemonic::Push | Mnemonic::Pop => operand_register(instruction, 0) == Some(Register::RBP),
        Mnemonic::Mov => {
            operand_register(instruction, 0) == Some(Register::RBP)
                && operand_register(instruction, 1) == Some(Register::RSP)
        }
        _ => false,
    }
}

fn register_bit(register: Register) -> u32 {
    let full = register.full_register();
    if full.is_gpr64() {
        1 << full.number()
    } else {
        0
    }
}

fn accessed_registers(instruction: &Instruction, accesses: &[OpAccess]) -> u32 {
    InstructionInfoFactory::new()
        .info(instruction)
        .used_registers()
        .iter()
        .filter(|used| accesses.contains(&used.access()))
        .fold(0, |registers, used| {
            registers | register_bit(used.register())
        })
}

fn still_held(instruction: &Instruction, holders: u32) -> u32 {
    if instruction.mnemonic() == Mnemonic::Mov
        && let (Some(target), Some(source)) = (
            operand_register(instruction, 0),
            operand_register(instruction, 1),
        )
        && source.size() == 8
        && holders & register_bit(source) != 0
    {
        return holders | register_bit(target);
    }
    let survivors = if instruction.mnemonic() == Mnemonic::Call {
        holders
            & PRESERVED_ACROSS_CALLS
                .into_iter()
                .fold(0, |set, register| set | register_bit(register))
    } else {
        holders
    };
    survivors & !accessed_registers(instruction, &WRITES)
}
