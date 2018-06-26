use cookie_factory::*;
use elements::{Info, Seek, SeekHead, SegmentElement, Cluster, Tracks,
  TrackEntry, Audio, Video, Colour, Projection, MasteringMetadata,
  SimpleBlock, Lacing};
use elements::{SilentTracks};
use super::ebml::{vint_size, gen_vint, gen_vid, gen_uint};
use serializer::ebml::{gen_u64, gen_f64_ref, gen_f64, EbmlSize};


pub fn seek_size(s: &Seek) -> u8 {
    // byte size of id (vid+size)+ data and position vid+size+int
    // FIXME: arbitrarily bad value
    vint_size(vint_size((s.id.len() + 10) as u64) as u64)
}

pub fn gen_segment<'a>(input: (&'a mut [u8], usize),
                       s: &SegmentElement)
                       -> Result<(&'a mut [u8], usize), GenError> {
    unimplemented!();
    /*do_gen!(input,
    gen_call!(gen_vid, 0x18538067) >>
    gen_call!(gen_vint, 4)
  )*/
}
pub fn gen_segment_header<'a>(input: (&'a mut [u8], usize), size: u64)
                       -> Result<(&'a mut [u8], usize), GenError> {
  do_gen!(input,
    gen_call!(gen_vid, 0x18538067) >>
    gen_call!(gen_vint, size)
  )
}

impl EbmlSize for Seek {
  fn capacity(&self) -> usize {
    self.id.size(0x53AB) + self.position.size(0x53AC)
  }
}

pub fn gen_seek<'a>(input: (&'a mut [u8], usize),
                    s: &Seek)
                    -> Result<(&'a mut [u8], usize), GenError> {
    //let capacity =  8 + 2 + vint_size(s.id.len() as u64) as u64 + s.id.len() as u64;
    let capacity = s.capacity() as u64;

    gen_ebml_master!(input,
    0x4DBB, vint_size(capacity),
    gen_ebml_binary!(0x53AB, s.id) >>
    gen_ebml_uint!(0x53AC, s.position, vint_size(s.position))
  )
}

impl EbmlSize for SeekHead {
  fn capacity(&self) -> usize {
    self.positions.iter().fold(0, |acc, seek| acc + seek.size(0x4DBB))
  }
}

pub fn gen_seek_head<'a>(input: (&'a mut [u8], usize),
                         s: &SeekHead)
                         -> Result<(&'a mut [u8], usize), GenError> {
    /*let capacity = s.positions.iter().fold(0u64, |acc, seek| {
        acc + 4 + 8 + 2 + vint_size(seek.id.len() as u64) as u64 + seek.id.len() as u64
    });*/
    let capacity = s.capacity() as u64;

    println!("gen_seek_head: calculated capacity: {} -> {} bytes", capacity, vint_size(capacity));

    let byte_capacity = vint_size(capacity as u64);
    gen_ebml_master!(input,
    0x114D9B74, byte_capacity,
    gen_many_ref!(&s.positions, gen_seek)
  )
}

impl EbmlSize for Info {
  fn capacity(&self) -> usize {
    self.segment_uid.size(0x73A4) + self.segment_filename.size(0x7384)
      + self.prev_uid.size(0x3CB923) + self.prev_filename.size(0x3C83AB)
      + self.next_uid.size(0x3EB923) + self.next_filename.size(0x3E83BB)
      //FIXME: chapter translate
      + self.segment_family.size(0x4444)
      + self.timecode_scale.size(0x2AD7B1) + self.duration.size(0x4489)
      + self.date_utc.size(0x4461) + self.title.size(0x7BA9)
      + self.muxing_app.size(0x4D80) + self.writing_app.size(0x5741)
  }
}

pub fn gen_info<'a>(input: (&'a mut [u8], usize),
                         i: &Info)
                         -> Result<(&'a mut [u8], usize), GenError> {
    let capacity = i.capacity();

    let byte_capacity = vint_size(capacity as u64);
    gen_ebml_master!(input,
      0x1549A966, byte_capacity,
      do_gen!(
           gen_opt!( i.segment_uid, gen_ebml_binary!(0x73A4) )
        >> gen_opt!( i.segment_filename, gen_ebml_str!(0x7384) )
        >> gen_opt!( i.prev_uid, gen_ebml_binary!(0x3CB923) )
        >> gen_opt!( i.prev_filename, gen_ebml_str!(0x3C83AB) )
        >> gen_opt!( i.next_uid, gen_ebml_binary!(0x3EB923) )
        >> gen_opt!( i.next_filename, gen_ebml_str!(0x3E83BB) )
        >> gen_opt!( i.segment_family, gen_ebml_binary!(0x4444) )
        //>> gen_opt!( i.chapter_translate, gen_chapter_translate )
        >> gen_call!(gen_u64, 0x2AD7B1, i.timecode_scale)
        >> gen_opt!( i.duration, gen_call!(gen_f64_ref, 0x4489) )
        >> gen_opt!( i.date_utc, gen_ebml_binary!(0x4461) )
        >> gen_opt!( i.title, gen_ebml_str!(0x7BA9) )
        >> gen_ebml_str!(0x4D80, i.muxing_app)
        >> gen_ebml_str!(0x5741, i.writing_app)
      )
    )
}


impl EbmlSize for Tracks {
  fn capacity(&self) -> usize {
    self.tracks.iter().fold(0, |acc, track| acc + track.size(0xAE))
  }
}

pub fn gen_tracks<'a>(input: (&'a mut [u8], usize),
                         t: &Tracks)
                         -> Result<(&'a mut [u8], usize), GenError> {
    let capacity = t.capacity();

    let byte_capacity = vint_size(capacity as u64);
    gen_ebml_master!(input,
      0x1654AE6B, byte_capacity,
      gen_many_ref!(&t.tracks, gen_track_entry)
    )
}

impl EbmlSize for TrackEntry {
  fn capacity(&self) -> usize {
    self.track_number.size(0xD7) + self.track_uid.size(0x73C5) + self.track_type.size(0x83)
      + self.flag_enabled.size(0xB9) + self.flag_default.size(0x88) + self.flag_forced.size(0x55AA)
      + self.flag_lacing.size(0x9C) + self.min_cache.size(0x6DE7) + self.max_cache.size(0x6DF8)
      + self.default_duration.size(0x23E383) + self.default_decoded_field_duration.size(0x234E7A)
      + self.track_timecode_scale.size(0x23314F) + self.track_offset.size(0x537F)
      + self.max_block_addition_id.size(0x55EE) + self.name.size(0x536E)
      + self.language.size(0x22B59C) + self.language_ietf.size(0x22B59D)
      + self.codec_id.size(0x86) + self.codec_private.size(0x63A2)
      + self.codec_name.size(0x258688) + self.attachment_link.size(0x7446)
      + self.codec_settings.size(0x3A9697) + self.codec_info_url.size(0x3B4040)
      + self.codec_download_url.size(0x26B240) + self.codec_decode_all.size(0xAA)
      + self.track_overlay.size(0x6FAB) + self.codec_delay.size(0x56AA)
      + self.seek_pre_roll.size(0x56BB)
      + self.video.size(0xE0)
      + self.audio.size(0xE1)
      + self.trick_track_uid.size(0xC0) + self.trick_track_segment_uid.size(0xC1)
      + self.trick_track_flag.size(0xC6) + self.trick_master_track_uid.size(0xC7)
      + self.trick_master_track_segment_uid.size(0xC4)
  }
}


pub fn gen_track_entry<'a>(input: (&'a mut [u8], usize),
                         t: &TrackEntry)
                         -> Result<(&'a mut [u8], usize), GenError> {
    let capacity = t.capacity();

    let byte_capacity = vint_size(capacity as u64);
    gen_ebml_master!(input,
      0xAE, byte_capacity,
      do_gen!(
           gen_ebml_uint!(0xD7,   t.track_number)
        >> gen_ebml_uint!(0x73C5, t.track_uid)
        >> gen_ebml_uint!(0x83,   t.track_type)
        >> gen_opt_copy!( t.flag_enabled, gen_ebml_uint!(0xB9))
        >> gen_opt_copy!( t.flag_default, gen_ebml_uint!(0x88))
        >> gen_opt_copy!( t.flag_forced,  gen_ebml_uint!(0x55AA))
        >> gen_opt_copy!( t.flag_lacing,  gen_ebml_uint!(0x9C))
        >> gen_opt_copy!( t.min_cache,    gen_ebml_uint!(0x6DE7))
        >> gen_opt_copy!( t.max_cache,    gen_ebml_uint!(0x6DF8))
        >> gen_opt_copy!( t.default_duration, gen_ebml_uint!(0x23E383))
        >> gen_opt_copy!( t.default_decoded_field_duration, gen_ebml_uint!(0x234E7A))
        >> gen_opt_copy!( t.track_timecode_scale, gen_call!(gen_f64, 0x23314F) )
        >> gen_opt_copy!( t.track_offset, gen_ebml_int!(0x537F) )
        >> gen_opt_copy!( t.max_block_addition_id, gen_ebml_uint!(0x55EE) )
        >> gen_opt!( t.name, gen_ebml_str!(0x536E) )
        >> gen_opt!( t.language, gen_ebml_str!(0x22B59C) )
        >> gen_opt!( t.language_ietf, gen_ebml_str!(0x22B59D) )
        >> gen_ebml_str!( 0x86, t.codec_id )
        >> gen_opt!( t.codec_private, gen_ebml_binary!(0x63A2) )
        >> gen_opt!( t.codec_name, gen_ebml_str!(0x258688) )
        >> gen_opt_copy!( t.attachment_link, gen_ebml_uint!(0x7446) )
        >> gen_opt!( t.codec_settings, gen_ebml_str!(0x3A9697) )
        >> gen_opt!( t.codec_info_url, gen_ebml_str!(0x3B4040) )
        >> gen_opt!( t.codec_download_url, gen_ebml_str!(0x26B240) )
        >> gen_opt_copy!( t.codec_decode_all, gen_ebml_uint!(0xAA) )
        >> gen_opt_copy!( t.track_overlay, gen_ebml_uint!(0x6FAB) )
        >> gen_opt_copy!( t.codec_delay, gen_ebml_uint!(0x56AA) )
        >> gen_opt_copy!( t.seek_pre_roll, gen_ebml_uint!(0x56BB) )
        >> gen_opt!( t.video, gen_call!(gen_track_entry_video) )
        >> gen_opt!( t.audio, gen_call!(gen_track_entry_audio) )
        >> gen_opt_copy!( t.trick_track_uid, gen_ebml_uint!(0xC0) )
        >> gen_opt!( t.trick_track_segment_uid, gen_ebml_binary!(0xC1) )
        >> gen_opt_copy!( t.trick_track_flag, gen_ebml_uint!(0xC6) )
        >> gen_opt_copy!( t.trick_master_track_uid, gen_ebml_uint!(0xC7) )
        >> gen_opt!( t.trick_master_track_segment_uid, gen_ebml_binary!(0xC4) )

      )
    )
}

impl EbmlSize for Audio {
  fn capacity(&self) -> usize {
    self.sampling_frequency.size(0xB5) + self.output_sampling_frequency.size(0x78B5) +
      self.channels.size(0x9F) + self.channel_positions.size(0x7D7B) +
      self.bit_depth.size(0x6264)
  }
}

pub fn gen_track_entry_audio<'a>(input: (&'a mut [u8], usize),
                         a: &Audio)
                         -> Result<(&'a mut [u8], usize), GenError> {
    let capacity = a.capacity();
    let byte_capacity = vint_size(capacity as u64);

    gen_ebml_master!(input,
      0xE1, byte_capacity,
      do_gen!(
           gen_call!( gen_f64, 0xB5,   a.sampling_frequency )
        >> gen_opt_copy!( a.output_sampling_frequency, gen_call!(gen_f64, 0x78B5))
        >> gen_opt_copy!( a.channels, gen_ebml_uint!(0x9F))
        >> gen_opt!( a.channel_positions, gen_ebml_binary!(0x7D7B))
        >> gen_opt_copy!( a.bit_depth, gen_ebml_uint!(0x6264))
      )
    )
}

impl EbmlSize for Video {
  fn capacity(&self) -> usize {
    self.flag_interlaced.size(0x9A) + self.field_order.size(0x9D) + self.stereo_mode.size(0x53B8) +
      self.alpha_mode.size(0x53C0) + self.old_stereo_mode.size(0x53B9) + self.pixel_width.size(0xB0) +
      self.pixel_height.size(0xBA) + self.pixel_crop_bottom.size(0x54AA) + self.pixel_crop_top.size(0x54BB) +
      self.pixel_crop_left.size(0x54CC) + self.pixel_crop_right.size(0x54DD) + self.display_width.size(0x54B0) +
      self.display_height.size(0x54BA) + self.display_unit.size(0x54B2) + self.aspect_ratio_type.size(0x54B3) +
      self.colour_space.size(0x2EB524) + self.gamma_value.size(0x2FB523) + self.frame_rate.size(0x2383E3) +
      self.colour.size(0x55B0) + self.projection.size(0x7670)
  }
}


pub fn gen_track_entry_video<'a>(input: (&'a mut [u8], usize),
                         v: &Video)
                         -> Result<(&'a mut [u8], usize), GenError> {
    let capacity = v.capacity();
    let byte_capacity = vint_size(capacity as u64);

    gen_ebml_master!(input,
      0xE0, byte_capacity,
      do_gen!(
           gen_opt_copy!( v.flag_interlaced, gen_ebml_uint!(0x9A))
        >> gen_opt_copy!( v.field_order, gen_ebml_uint!(0x9D))
        >> gen_opt_copy!( v.stereo_mode, gen_ebml_uint!(0x53B8))
        >> gen_opt_copy!( v.alpha_mode, gen_ebml_uint!(0x53C0))
        >> gen_opt_copy!( v.old_stereo_mode, gen_ebml_uint!(0x53B9))
        >> gen_ebml_uint!(0xB0, v.pixel_width)
        >> gen_ebml_uint!(0xBA, v.pixel_height)
        >> gen_opt_copy!( v.pixel_crop_bottom, gen_ebml_uint!(0x54AA))
        >> gen_opt_copy!( v.pixel_crop_top, gen_ebml_uint!(0x54BB))
        >> gen_opt_copy!( v.pixel_crop_left, gen_ebml_uint!(0x54CC))
        >> gen_opt_copy!( v.pixel_crop_right, gen_ebml_uint!(0x54DD))
        >> gen_opt_copy!( v.display_width, gen_ebml_uint!(0x54B0))
        >> gen_opt_copy!( v.display_height, gen_ebml_uint!(0x54BA))
        >> gen_opt_copy!( v.display_unit, gen_ebml_uint!(0x54B2))
        >> gen_opt_copy!( v.aspect_ratio_type, gen_ebml_uint!(0x54B3))
        >> gen_opt!( v.colour_space, gen_ebml_binary!(0x2EB524))
        >> gen_opt_copy!( v.gamma_value, gen_call!(gen_f64, 0x2FB523))
        >> gen_opt_copy!( v.frame_rate, gen_call!(gen_f64, 0x2383E3))
        >> gen_opt!( v.colour, gen_call!(gen_track_entry_video_colour) )
        >> gen_opt!( v.projection, gen_call!(gen_track_entry_video_projection) )
      )
    )
}

impl EbmlSize for Colour {
  fn capacity(&self) -> usize {
    self.matrix_coefficients.size(0x55B1) + self.bits_per_channel.size(0x55B2) + self.chroma_subsampling_horz.size(0x55B3) +
      self.chroma_subsampling_vert.size(0x55B4) + self.cb_subsampling_horz.size(0x55B5) + self.cb_subsampling_vert.size(0x55B6) +
      self.chroma_siting_horz.size(0x55B7) + self.chroma_siting_vert.size(0x55B8) + self.range.size(0x55B9) +
      self.transfer_characteristics.size(0x55BA) + self.primaries.size(0x55BB) + self.max_cll.size(0x55BC) +
      self.max_fall.size(0x55BD) + self.mastering_metadata.size(0x55D0)
  }
}

pub fn gen_track_entry_video_colour<'a>(input: (&'a mut [u8], usize),
                         c: &Colour)
                         -> Result<(&'a mut [u8], usize), GenError> {
    let capacity = c.capacity();
    let byte_capacity = vint_size(capacity as u64);

    gen_ebml_master!(input,
      0x55B0, byte_capacity,
      do_gen!(
           gen_opt_copy!( c.matrix_coefficients, gen_ebml_uint!(0x55B1))
        >> gen_opt_copy!( c.bits_per_channel, gen_ebml_uint!(0x55B2))
        >> gen_opt_copy!( c.chroma_subsampling_horz, gen_ebml_uint!(0x55B3))
        >> gen_opt_copy!( c.chroma_subsampling_vert, gen_ebml_uint!(0x55B4))
        >> gen_opt_copy!( c.cb_subsampling_horz, gen_ebml_uint!(0x55B5))
        >> gen_opt_copy!( c.cb_subsampling_vert, gen_ebml_uint!(0x55B6))
        >> gen_opt_copy!( c.chroma_siting_horz, gen_ebml_uint!(0x55B7))
        >> gen_opt_copy!( c.chroma_siting_vert, gen_ebml_uint!(0x55B8))
        >> gen_opt_copy!( c.range, gen_ebml_uint!(0x55B9))
        >> gen_opt_copy!( c.transfer_characteristics, gen_ebml_uint!(0x55BA))
        >> gen_opt_copy!( c.primaries, gen_ebml_uint!(0x55BB))
        >> gen_opt_copy!( c.max_cll, gen_ebml_uint!(0x55BC))
        >> gen_opt_copy!( c.max_fall, gen_ebml_uint!(0x55BD))
        >> gen_opt!( c.mastering_metadata, gen_call!(gen_track_entry_video_colour_mastering_metadata) )
      )
    )
}


impl EbmlSize for MasteringMetadata {
  fn capacity(&self) -> usize {
    self.primary_r_chromaticity_x.size(0x55D1) + self.primary_r_chromaticity_y.size(0x55D2) +
      self.primary_g_chromaticity_x.size(0x55D3) + self.primary_g_chromaticity_y.size(0x55D4) +
      self.primary_b_chromaticity_x.size(0x55D5) + self.primary_b_chromaticity_y.size(0x55D6) +
      self.white_point_chromaticity_x.size(0x55D7) + self.white_point_chromaticity_y.size(0x55D8) +
      self.luminance_max.size(0x55D9) + self.luminance_min.size(0x55DA)
  }
}

pub fn gen_track_entry_video_colour_mastering_metadata<'a>(input: (&'a mut [u8], usize),
                         m: &MasteringMetadata)
                         -> Result<(&'a mut [u8], usize), GenError> {
    let capacity = m.capacity();
    let byte_capacity = vint_size(capacity as u64);

    gen_ebml_master!(input,
      0x55D0, byte_capacity,
      do_gen!(
           gen_opt_copy!( m.primary_r_chromaticity_x, gen_call!(gen_f64, 0x55D1))
        >> gen_opt_copy!( m.primary_r_chromaticity_y, gen_call!(gen_f64, 0x55D2))
        >> gen_opt_copy!( m.primary_g_chromaticity_x, gen_call!(gen_f64, 0x55D3))
        >> gen_opt_copy!( m.primary_g_chromaticity_y, gen_call!(gen_f64, 0x55D4))
        >> gen_opt_copy!( m.primary_b_chromaticity_x, gen_call!(gen_f64, 0x55D5))
        >> gen_opt_copy!( m.primary_b_chromaticity_y, gen_call!(gen_f64, 0x55D6))
        >> gen_opt_copy!( m.white_point_chromaticity_y, gen_call!(gen_f64, 0x55D7))
        >> gen_opt_copy!( m.white_point_chromaticity_y, gen_call!(gen_f64, 0x55D8))
        >> gen_opt_copy!( m.luminance_max, gen_call!(gen_f64, 0x55D9))
        >> gen_opt_copy!( m.luminance_min, gen_call!(gen_f64, 0x55DA))
      )
    )
}


impl EbmlSize for Projection {
  fn capacity(&self) -> usize {
    self.projection_type.size(0x7671) + self.projection_private.size(0x7672) +
      self.projection_pose_yaw.size(0x7673) + self.projection_pose_pitch.size(0x7674) +
      self.projection_pose_roll.size(0x7675)
  }
}


pub fn gen_track_entry_video_projection<'a>(input: (&'a mut [u8], usize),
                         p: &Projection)
                         -> Result<(&'a mut [u8], usize), GenError> {
    let capacity = p.capacity();
    let byte_capacity = vint_size(capacity as u64);

    gen_ebml_master!(input,
      0x7670, byte_capacity,
      do_gen!(
           gen_ebml_uint!(0x7671, p.projection_type)
        >> gen_opt!( p.projection_private, gen_ebml_binary!(0x7672))
        >> gen_call!( gen_f64, 0x7673, p.projection_pose_yaw )
        >> gen_call!( gen_f64, 0x7674, p.projection_pose_pitch )
        >> gen_call!( gen_f64, 0x7675, p.projection_pose_roll )
      )
    )
}


#[macro_export]
macro_rules! my_gen_many (
    (($i:expr, $idx:expr), $l:expr, $f:ident) => (
        $l.into_iter().fold(
            Ok(($i,$idx)),
            |r,v| {
                match r {
                    Err(e) => Err(e),
                    Ok(x) => { $f(x, v) },
                }
            }
        )
    );
    (($i:expr, $idx:expr), $l:expr, $f:ident!( $($args:tt)* )) => (
        $l.into_iter().fold(
            Ok(($i,$idx)),
            |r,v| {
                match r {
                    Err(e) => Err(e),
                    Ok(x) => {
                      let (i, idx) = x;
                      $f!((i, idx), $($args)*, v)
                    },
                }
            }
        )
    );
);

#[macro_export]
macro_rules! my_gen_many_ref (
    (($i:expr, $idx:expr), $l:expr, $f:ident) => (
        $l.into_iter().fold(
            Ok(($i,$idx)),
            |r,v| {
                match r {
                    Err(e) => Err(e),
                    Ok(x) => { $f(x, *v) },
                }
            }
        )
    );
    (($i:expr, $idx:expr), $l:expr, $f:ident!( $($args:tt)* )) => (
        $l.into_iter().fold(
            Ok(($i,$idx)),
            |r,v| {
                match r {
                    Err(e) => Err(e),
                    Ok(x) => {
                      let (i, idx) = x;
                      $f!((i, idx), $($args)*, *v)
                    },
                }
            }
        )
    );
);

impl<'a> EbmlSize for Cluster<'a> {
  fn capacity(&self) -> usize {
    self.timecode.size(0xE7) + self.silent_tracks.size(0x5854) + self.position.size(0xA7) +
      self.prev_size.size(0xAB) + self.simple_block.size(0xA3) +
      // TODO: implement for BlockGroup
      // self.block_group.size(0xA0) +
      self.encrypted_block.size(0xAF)
  }
}

pub fn gen_cluster<'a>(input: (&'a mut [u8], usize),
                         c: &Cluster)
                         -> Result<(&'a mut [u8], usize), GenError> {

    let capacity = c.capacity();
    let byte_capacity = vint_size(capacity as u64);
    gen_ebml_master!(input,
      0x1F43B675, byte_capacity,
      do_gen!(
           gen_ebml_uint!(0xE7, c.timecode)
        >> gen_opt!( c.silent_tracks, gen_call!(gen_silent_tracks) )
        >> gen_opt_copy!( c.position, gen_ebml_uint!(0xA7) )
        >> gen_opt_copy!( c.prev_size, gen_ebml_uint!(0xAB) )
        >> my_gen_many!( &c.simple_block, gen_ebml_binary!( 0xA3 ) )
        // TODO: implement for BlockGroup
        //>> my_gen_many!( &c.block_group, gen_block_group)
        >> gen_opt!( c.encrypted_block, gen_ebml_binary!( 0xAF ) )
      )
    )
}

impl EbmlSize for SilentTracks {
  fn capacity(&self) -> usize {
    self.numbers.size(0x58D7)
  }
}

pub fn gen_silent_tracks<'a>(input: (&'a mut [u8], usize),
                         s: &SilentTracks)
                         -> Result<(&'a mut [u8], usize), GenError> {

    let capacity = s.capacity();
    let byte_capacity = vint_size(capacity as u64);
    gen_ebml_master!(input,
      0x5854, byte_capacity,
      do_gen!(
        my_gen_many_ref!( &s.numbers, gen_ebml_uint!(0x58D7))
      )
    )
}

pub fn gen_simple_block_header<'a>(input: (&'a mut [u8], usize),
                                   s: &SimpleBlock)
                                   -> Result<(&'a mut [u8], usize), GenError> {


  let mut flags = 0u8;

  if s.keyframe {
    flags |= 0b0001u8;
  }

  if s.invisible {
    flags |= 0b00010000u8;
  }

  flags |= match s.lacing {
    Lacing::None      => 0u8,
    Lacing::Xiph      => 0b00100000u8,
    Lacing::FixedSize => 0b01000000u8,
    Lacing::EBML      => 0b01100000u8,
  };

  if s.discardable {
    flags |= 0b10000000u8;
  }

  do_gen!(input,
    gen_call!(gen_vint, s.track_number) >>
    gen_be_i16!(s.timecode) >>
    gen_be_u8!(flags)
  )
}
pub fn gen_laced_frames<'a>(input: (&'a mut [u8], usize),
                            lacing: Lacing,
                            frames: &[&[u8]])
                            -> Result<(&'a mut [u8], usize), GenError> {
  match lacing {
    Lacing::None      => Err(GenError::NotYetImplemented),
    Lacing::Xiph      => gen_xiph_laced_frames(input, frames),
    Lacing::EBML      => gen_ebml_laced_frames(input, frames),
    Lacing::FixedSize => gen_fixed_size_laced_frames(input, frames),
  }
}

pub fn gen_xiph_laced_frames<'a>(input: (&'a mut [u8], usize),
                            frames: &[&[u8]])
                            -> Result<(&'a mut [u8], usize), GenError> {
  if frames.len() == 0 {
    return Err(GenError::NotYetImplemented);
  }

  /*
  let sizes: Vec<usize> = frames.iter().map(|frame| frame.len()).collect();
  do_gen!(input,
    gen_be_u8!((frames.len() - 1) as u8) >>

  )
  */
  Err(GenError::NotYetImplemented)
}

pub fn gen_ebml_laced_frames<'a>(input: (&'a mut [u8], usize),
                            frames: &[&[u8]])
                            -> Result<(&'a mut [u8], usize), GenError> {
  Err(GenError::NotYetImplemented)
}

pub fn gen_fixed_size_laced_frames<'a>(input: (&'a mut [u8], usize),
                            frames: &[&[u8]])
                            -> Result<(&'a mut [u8], usize), GenError> {
  Err(GenError::NotYetImplemented)
}

/*
impl<'a> EbmlSize for BlockGroup<'a> {
  fn capacity(&self) -> usize {
    self.timecode.size(0xE7) + self.silent_tracks.size(0x5854) + self.position.size(0xA7) +
      self.prev_size.size(0xAB) + self.simple_block.size(0xA3) + self.block_group.size(0xA0) +
      self.encrypted_block.size(0xAF)
  }
}

pub fn gen_block_group<'a>(input: (&'a mut [u8], usize),
                         b: &Blockgroup)
                         -> Result<(&'a mut [u8], usize), GenError> {

    let capacity = c.capacity();
    let byte_capacity = vint_size(capacity as u64);
    gen_ebml_master!(input,
      0x1F43B675, byte_capacity,
      do_gen!(
           gen_ebml_uint!(0xE7, c.timecode)
        >> gen_opt!( c.silent_tracks, gen_call!(gen_silent_tracks) )
        >> gen_opt_copy!( c.position, gen_ebml_uint!(0xA7) )
        >> gen_opt_copy!( c.prev_size, gen_ebml_uint!(0xAB) )
        >> my_gen_many!( &c.simple_block, gen_ebml_binary!( 0xA3 ) )
        >> my_gen_many!( &c.block_group, gen_block_group)
        >> gen_opt!( &c.encrypted_block, gen_ebml_binary!( 0xAF ) )
      )
    )
}
*/

#[cfg(test)]
mod tests {
    use super::*;
    use nom::*;
    use std::iter::repeat;

    fn test_seek_head_serializer(mut seeks: Vec<(u64, Vec<u8>)>) -> bool {
        println!("testing for {:?}", seeks);

        let mut should_fail = false;
        if seeks.len() == 0 {
            should_fail = true;
        }

        for &(_, ref id) in seeks.iter() {
            println!("id: {}", id.to_hex(16));
            if id.len() == 0 {
                println!("id is empty, returning");
                return true;
                //should_fail = true;
            }
        }

        if should_fail {
            println!("the parser should fail");
        }

        let capacity = seeks.iter().fold(0, |acc, &(_, ref v)| acc + 8 + v.len() + 100);
        println!("defining capacity as {}", capacity);

        let mut data = Vec::with_capacity(capacity);
        data.extend(repeat(0).take(capacity));

        let seek_head = SeekHead {
            positions: seeks.iter()
                .cloned()
                .map(|(position, id)| {
                    Seek {
                        id: id,
                        position: position,
                    }
                })
                .collect(),
        };

        let ser_res = {
            let gen_res = gen_seek_head((&mut data[..], 0), &seek_head);
            println!("gen_res: {:?}", gen_res);
            if let Err(e) = gen_res {
                println!("gen_res is error: {:?}", e);
                println!("should fail: {:?}", should_fail);
                return should_fail;
                /*if should_fail {
          println!("should fail");
          return true;
        }*/
            }
        };

        println!("ser_res: {:?}", ser_res);

        let parse_res = ::elements::segment_element(&data[..]);
        println!("parse_res: {:?}", parse_res);
        match parse_res {
            Ok((rest, SegmentElement::SeekHead(o))) => {
                if should_fail {
                    println!("parser should have failed on input for {:?}", seek_head);
                    println!("{}", (&data[..]).to_hex(16));
                    return false;
                }

                assert_eq!(seek_head, o);
                return true;
            }
            e => {
                if should_fail {
                    return true;
                }

                panic!(format!("parse error: {:?} for input: {:?}", e, seeks))
            }
        }

        false
    }

    quickcheck! {
    fn test_seek_head(seeks: Vec<(u64, Vec<u8>)>) -> bool {
      test_seek_head_serializer(seeks)
    }
  }
}
