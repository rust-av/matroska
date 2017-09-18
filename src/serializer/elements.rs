use cookie_factory::*;
use elements::{Seek, SeekHead};
use super::ebml::{vint_size, gen_vint, gen_vid, gen_uint};


pub fn seek_size(s: &Seek) -> u8 {
    // byte size of id (vid+size)+ data and position vid+size+int
    // FIXME: arbitrarily bad value
    vint_size(vint_size((s.id.len() + 10) as u64) as u64)
}

pub fn gen_seek<'a>(input: (&'a mut [u8], usize),
                    s: &Seek)
                    -> Result<(&'a mut [u8], usize), GenError> {
    gen_ebml_master!(input,
    0x4DBB, 4,
    gen_ebml_binary!(0x53AB, s.id) >>
    gen_ebml_uint!(0x53AC, s.position, 2)
  )
}

pub fn gen_seek_head<'a>(input: (&'a mut [u8], usize),
                         s: &SeekHead)
                         -> Result<(&'a mut [u8], usize), GenError> {
    gen_ebml_master!(input,
    0x114D9B74, 4,
    gen_many_ref!(&s.positions, gen_seek)
  )
}
