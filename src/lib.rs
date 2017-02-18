extern crate byteorder;
use byteorder::{BigEndian, ByteOrder};
use std::fmt::{Debug, Result, Formatter};

#[derive(Default)]
pub struct MobiFile<'a> {
    pub data: &'a mut [u8],
    pub length: usize,
    pub num_sections: u32,
    pub mobi_section_indice: Vec<u32>, // indices of sections that are mobi headers
}

impl<'a> MobiFile<'a> {
    pub fn read_bytes(&self, offset: usize, length: usize) -> &[u8] {
        &self.data[offset..offset+length]
    }

    fn read_short(&self, offset: usize) -> u16 {
        let bytes = &self.data[offset..offset+2];
        BigEndian::read_u16(bytes)
    }

    // fn write_short(&mut self, offset: usize, short: u16) {
    //     BigEndian::write_u16(&mut self.data[offset..offset+2], short);
    // }

    fn read_long(&self, offset: usize) -> u32 {
        let bytes = &self.data[offset..offset+4];
        BigEndian::read_u32(bytes)
    }

    fn write_long(&mut self, offset: usize, value: u32) {
        BigEndian::write_u32(&mut self.data[offset..offset+4], value)
    }

    fn move_ahead(&mut self, src_offset: usize, dst_offset: usize, length: u32) {
        for i in 0..length as usize {
            self.data[dst_offset + i] = self.data[src_offset + i];
        }
    }
}


impl<'a> MobiFile<'a> {
    pub fn new(data: &'a mut[u8], length: usize) -> MobiFile<'a> {
        let mut me = MobiFile {
            data: data,
            length: length,
            ..Default::default()
        };

        {
            let bookmobi:&[u8] = b"BOOKMOBI";
            if me.read_bytes(0x3c, 8) != bookmobi {
                panic!("Invalid file format");
            }
        }

        me.num_sections = me.read_short(0x4c) as u32;

        {
            me.mobi_section_indice = vec![0];
            let boundaries_before_mobi = (0..me.num_sections).filter(|&i| {
                let is_boundary = me.get_section(i).is_boundary();
                if is_boundary {
                    let next = me.get_section(i+1);
                    if next.is_mobi() {
                        return true;
                    }
                }
                return false;
            }).collect::<Vec<_>>();
            me.mobi_section_indice.extend(boundaries_before_mobi.iter().map(|i| i + 1));
        }

        for index in me.mobi_section_indice.iter() {
            let section = me.get_section(*index);
            println!("Header version {} found", section.get_version());
            println!("   Title: {}", section.get_full_name());
            println!("   Encoding: {}", section.get_encoding());
        }
        println!("==========================================");
        me
    }

    fn get_section_offset(&self, index: u32) -> usize {
        self.read_long(0x4e + 8 * index as usize) as usize
    }

    fn get_section_addr(&self, index: u32) -> [usize; 2] {
        let start = self.get_section_offset(index);
        let end = if index == self.num_sections - 1 {
            self.length
        } else {
            self.get_section_offset(index + 1)
        };
        return [start, end];
    }

    fn get_section_length(&self, index: u32) -> usize {
        let tmp = self.get_section_addr(index);
        (tmp[1] - tmp[0])
    }

    fn get_section(&self, index: u32) -> Section {
        let range = self.get_section_addr(index);
        Section {
            data: &self.data[range[0]..range[1]],
            index: index,
            start: range[0],
        }
    }

    fn get_source_sections(&self) -> Vec<u32> {
        self.mobi_section_indice.iter()
            .flat_map(|&index| self.get_section(index).get_source_section_indice())
            .collect()
    }

    fn set_section_addr_offset(&mut self, index: u32, offset: usize) {
        self.write_long(0x4e + 8 * index as usize, offset as u32)
    }

    pub fn remove_sources(&mut self) -> usize {
        let source_sections = self.get_source_sections();

        // rewrite sections
        let mut delta = 0;
        for i in 0..self.num_sections {
            let length = self.get_section_length(i);
            let offset = self.get_section_offset(i);

            self.set_section_addr_offset(i, offset - delta);

            if source_sections.contains(&i) {
                delta += length;
                println!("Clear data from section {} with length {}", i, length);
            } else {
                self.move_ahead(offset, offset - delta, length as u32);
            }
        }


        self.clear_section_sources();
        self.length = self.length - delta;
        self.length
    }
}

impl<'a> MobiFile<'a> {
    fn clear_section_sources(&mut self) {
        for index in self.mobi_section_indice.clone().iter() {
            let section_offset = self.get_section_offset(*index);
            self.write_long(section_offset + 0xe0, 0xffffffff);
            self.write_long(section_offset + 0xe4, 0); 
        };
    }
}

struct Section<'a> {
    data: &'a [u8],
    index: u32,
    start: usize,
}
impl<'a> Debug for Section<'a> {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "<Section {} at {:#010x}>", self.index, self.start)
    }
}

impl<'a> Section<'a> {
    fn read_bytes(&self, offset: usize, length: usize) -> &[u8] {
        &self.data[offset..offset+length]
    }

    fn read_long(&self, offset: usize) -> u32 {
        let bytes = &self.data[offset..offset+4];
        BigEndian::read_u32(bytes)
    }

    fn is_boundary(&self) -> bool {
        let boundary:&[u8] = b"BOUNDARY";
        self.data == boundary
    }

    fn is_mobi(&self) -> bool {
        if self.data.len() < 0x10 + 4 {
            return false;
        }
        let mobi:&[u8] = b"MOBI";
        self.read_bytes(0x10, 4) == mobi
    }

    fn get_version(&self) -> u32 {
        assert!(self.is_mobi());

        self.read_long(0x24)
    }
    
    fn get_source_section_indice(&self) -> Vec<u32> {
        assert!(self.is_mobi());

        let source_index = self.read_long(0xe0);
        let source_count = self.read_long(0xe4);

        if source_index == 0xffffffff || source_count == 0 {
            return Vec::new();
        }
        (source_index..source_index + source_count).collect()
    }

    fn get_encoding(&self) -> u32 {
        assert!(self.is_mobi());

        self.read_long(0x1c)
    }

    fn get_full_name(&self) -> String {
        assert!(self.is_mobi());

        let offset = self.read_long(0x54) as usize;
        let length = self.read_long(0x58) as usize;
        String::from_utf8(self.read_bytes(offset, length).to_vec()).unwrap()
    }
}

#[cfg(target_os = "emscripten")]
#[no_mangle]
pub fn process_mobi_file(data: &mut [u8]) -> usize {
    println!("Data ptr {:?}", data.as_ptr());
    let length = data.len();
    let mut file = MobiFile::new(data, length);

    println!("Removing sources...");
    let length = file.remove_sources();
    length
}
