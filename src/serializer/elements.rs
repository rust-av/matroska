use cookie_factory::gen::set_be_u8;
use cookie_factory::GenError;

use crate::{
    elements::{
        Audio, Cluster, Colour, Info, Lacing, MasteringMetadata, Projection, Seek, SeekHead,
        SilentTracks, SimpleBlock, TrackEntry, Tracks, Video,
    },
    serializer::cookie_utils::{gen_many, gen_opt, gen_opt_copy, set_be_i16, tuple},
    serializer::ebml::{
        gen_ebml_binary, gen_ebml_int, gen_ebml_master, gen_ebml_str, gen_ebml_uint,
        gen_ebml_uint_l, gen_f64, gen_f64_ref, gen_vid, gen_vint, vint_size, EbmlSize,
    },
};

pub(crate) fn gen_segment_header_unknown_size(
) -> impl Fn((&mut [u8], usize)) -> Result<(&mut [u8], usize), GenError> {
    move |input| tuple((gen_vid(0x18538067), |i| set_be_u8(i, 0xFF)))(input)
}

impl EbmlSize for Seek {
    fn capacity(&self) -> usize {
        self.id.size(0x53AB) + self.position.size(0x53AC)
    }
}

fn gen_seek<'a>(
    s: &'a Seek,
) -> impl Fn((&'a mut [u8], usize)) -> Result<(&'a mut [u8], usize), GenError> {
    move |input| {
        gen_ebml_master(
            0x4DBB,
            vint_size(s.capacity() as u64),
            tuple((
                gen_ebml_binary(0x53AB, &s.id),
                gen_ebml_uint_l(0x53AC, s.position, 8),
            )),
        )(input)
    }
}

impl EbmlSize for SeekHead {
    fn capacity(&self) -> usize {
        self.positions
            .iter()
            .fold(0, |acc, seek| acc + seek.size(0x4DBB))
    }
}

pub(crate) fn gen_seek_head<'a>(
    s: &'a SeekHead,
) -> impl Fn((&'a mut [u8], usize)) -> Result<(&'a mut [u8], usize), GenError> {
    move |input| {
        let byte_capacity = vint_size(s.capacity() as u64);
        gen_ebml_master(0x114D9B74, byte_capacity, gen_many(&s.positions, gen_seek))(input)
    }
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

pub(crate) fn gen_info<'a>(
    i: &'a Info,
) -> impl Fn((&'a mut [u8], usize)) -> Result<(&'a mut [u8], usize), GenError> {
    move |input| {
        let byte_capacity = vint_size(i.capacity() as u64);
        gen_ebml_master(
            0x1549A966,
            byte_capacity,
            tuple((
                gen_opt(i.segment_uid.as_ref(), |v| gen_ebml_binary(0x73A4, v)),
                gen_opt(i.segment_filename.as_ref(), |v| gen_ebml_str(0x7384, v)),
                gen_opt(i.prev_uid.as_ref(), |v| gen_ebml_binary(0x3CB923, v)),
                gen_opt(i.prev_filename.as_ref(), |v| gen_ebml_str(0x3C83AB, v)),
                gen_opt(i.next_uid.as_ref(), |v| gen_ebml_binary(0x3EB923, v)),
                gen_opt(i.next_filename.as_ref(), |v| gen_ebml_str(0x3E83BB, v)),
                gen_opt(i.segment_family.as_ref(), |v| gen_ebml_binary(0x4444, v)),
                gen_ebml_uint(0x2AD7B1, i.timecode_scale),
                gen_opt(i.duration.as_ref(), |v| gen_f64_ref(0x4489, v)),
                gen_opt(i.date_utc.as_ref(), |v| gen_ebml_binary(0x4461, v)),
                gen_opt(i.title.as_ref(), |v| gen_ebml_str(0x7BA9, v)),
                gen_ebml_str(0x4D80, &i.muxing_app),
                gen_ebml_str(0x5741, &i.writing_app),
            )),
        )(input)
    }
}

impl EbmlSize for Tracks {
    fn capacity(&self) -> usize {
        self.tracks
            .iter()
            .fold(0, |acc, track| acc + track.size(0xAE))
    }
}

pub(crate) fn gen_tracks<'a>(
    t: &'a Tracks,
) -> impl Fn((&'a mut [u8], usize)) -> Result<(&'a mut [u8], usize), GenError> {
    move |input| {
        let byte_capacity = vint_size(t.capacity() as u64);
        gen_ebml_master(
            0x1654AE6B,
            byte_capacity,
            gen_many(&t.tracks, gen_track_entry),
        )(input)
    }
}

impl EbmlSize for TrackEntry {
    fn capacity(&self) -> usize {
        self.track_number.size(0xD7)
            + self.track_uid.size(0x73C5)
            + self.track_type.size(0x83)
            + self.flag_enabled.size(0xB9)
            + self.flag_default.size(0x88)
            + self.flag_forced.size(0x55AA)
            + self.flag_lacing.size(0x9C)
            + self.min_cache.size(0x6DE7)
            + self.max_cache.size(0x6DF8)
            + self.default_duration.size(0x23E383)
            + self.default_decoded_field_duration.size(0x234E7A)
            + self.track_timecode_scale.size(0x23314F)
            + self.track_offset.size(0x537F)
            + self.max_block_addition_id.size(0x55EE)
            + self.name.size(0x536E)
            + self.language.size(0x22B59C)
            + self.language_ietf.size(0x22B59D)
            + self.codec_id.size(0x86)
            + self.codec_private.size(0x63A2)
            + self.codec_name.size(0x258688)
            + self.attachment_link.size(0x7446)
            + self.codec_settings.size(0x3A9697)
            + self.codec_info_url.size(0x3B4040)
            + self.codec_download_url.size(0x26B240)
            + self.codec_decode_all.size(0xAA)
            + self.track_overlay.size(0x6FAB)
            + self.codec_delay.size(0x56AA)
            + self.seek_pre_roll.size(0x56BB)
            + self.video.size(0xE0)
            + self.audio.size(0xE1)
            + self.trick_track_uid.size(0xC0)
            + self.trick_track_segment_uid.size(0xC1)
            + self.trick_track_flag.size(0xC6)
            + self.trick_master_track_uid.size(0xC7)
            + self.trick_master_track_segment_uid.size(0xC4)
    }
}

fn gen_track_entry<'a>(
    t: &'a TrackEntry,
) -> impl Fn((&'a mut [u8], usize)) -> Result<(&'a mut [u8], usize), GenError> {
    move |input| {
        let capacity = t.capacity();

        let byte_capacity = vint_size(capacity as u64);
        gen_ebml_master(
            0xAE,
            byte_capacity,
            tuple((
                gen_ebml_uint(0xD7, t.track_number),
                gen_ebml_uint(0x73C5, t.track_uid),
                gen_ebml_uint(0x83, t.track_type),
                gen_opt_copy(t.flag_enabled, |v| gen_ebml_uint(0xB9, v)),
                gen_opt_copy(t.flag_default, |v| gen_ebml_uint(0x88, v)),
                gen_opt_copy(t.flag_forced, |v| gen_ebml_uint(0x55AA, v)),
                gen_opt_copy(t.flag_lacing, |v| gen_ebml_uint(0x9C, v)),
                gen_opt_copy(t.min_cache, |v| gen_ebml_uint(0x6DE7, v)),
                gen_opt_copy(t.max_cache, |v| gen_ebml_uint(0x6DF8, v)),
                gen_opt_copy(t.default_duration, |v| gen_ebml_uint(0x23E383, v)),
                gen_opt_copy(t.default_decoded_field_duration, |v| {
                    gen_ebml_uint(0x234E7A, v)
                }),
                gen_opt_copy(t.track_timecode_scale, |v| gen_f64(0x23314F, v)),
                gen_opt_copy(t.track_offset, |v| gen_ebml_int(0x537F, v)),
                gen_opt_copy(t.max_block_addition_id, |v| gen_ebml_uint(0x55EE, v)),
                gen_opt(t.name.as_ref(), |v| gen_ebml_str(0x536E, v)),
                gen_opt(t.language.as_ref(), |v| gen_ebml_str(0x22B59C, v)),
                gen_opt(t.language_ietf.as_ref(), |v| gen_ebml_str(0x22B59D, v)),
                gen_ebml_str(0x86, &t.codec_id),
                gen_opt(t.codec_private.as_ref(), |v| gen_ebml_binary(0x63A2, v)),
                gen_opt(t.codec_name.as_ref(), |v| gen_ebml_str(0x258688, v)),
                gen_opt_copy(t.attachment_link, |v| gen_ebml_uint(0x7446, v)),
                gen_opt(t.codec_settings.as_ref(), |v| gen_ebml_str(0x3A9697, v)),
                gen_opt(t.codec_info_url.as_ref(), |v| gen_ebml_str(0x3B4040, v)),
                gen_opt(t.codec_download_url.as_ref(), |v| gen_ebml_str(0x26B240, v)),
                gen_opt_copy(t.codec_decode_all, |v| gen_ebml_uint(0xAA, v)),
                gen_opt_copy(t.track_overlay, |v| gen_ebml_uint(0x6FAB, v)),
                gen_opt_copy(t.codec_delay, |v| gen_ebml_uint(0x56AA, v)),
                gen_opt_copy(t.seek_pre_roll, |v| gen_ebml_uint(0x56BB, v)),
                gen_opt(t.video.as_ref(), gen_track_entry_video),
                gen_opt(t.audio.as_ref(), gen_track_entry_audio),
                gen_opt_copy(t.trick_track_uid, |v| gen_ebml_uint(0xC0, v)),
                gen_opt(t.trick_track_segment_uid.as_ref(), |v| {
                    gen_ebml_binary(0xC1, v)
                }),
                gen_opt_copy(t.trick_track_flag, |v| gen_ebml_uint(0xC6, v)),
                gen_opt_copy(t.trick_master_track_uid, |v| gen_ebml_uint(0xC7, v)),
                gen_opt(t.trick_master_track_segment_uid.as_ref(), |v| {
                    gen_ebml_binary(0xC4, v)
                }),
            )),
        )(input)
    }
}

impl EbmlSize for Audio {
    fn capacity(&self) -> usize {
        self.sampling_frequency.size(0xB5)
            + self.output_sampling_frequency.size(0x78B5)
            + self.channels.size(0x9F)
            + self.channel_positions.size(0x7D7B)
            + self.bit_depth.size(0x6264)
    }
}

fn gen_track_entry_audio<'a>(
    a: &'a Audio,
) -> impl Fn((&'a mut [u8], usize)) -> Result<(&'a mut [u8], usize), GenError> {
    move |input| {
        let byte_capacity = vint_size(a.capacity() as u64);

        gen_ebml_master(
            0xE1,
            byte_capacity,
            tuple((
                gen_f64(0xB5, a.sampling_frequency),
                gen_opt_copy(a.output_sampling_frequency, |v| gen_f64(0x78B5, v)),
                gen_ebml_uint(0x9F, a.channels),
                gen_opt(a.channel_positions.as_ref(), |v| gen_ebml_binary(0x7D7B, v)),
                gen_opt_copy(a.bit_depth, |v| gen_ebml_uint(0x6264, v)),
            )),
        )(input)
    }
}

impl EbmlSize for Video {
    fn capacity(&self) -> usize {
        self.flag_interlaced.size(0x9A)
            + self.field_order.size(0x9D)
            + self.stereo_mode.size(0x53B8)
            + self.alpha_mode.size(0x53C0)
            + self.old_stereo_mode.size(0x53B9)
            + self.pixel_width.size(0xB0)
            + self.pixel_height.size(0xBA)
            + self.pixel_crop_bottom.size(0x54AA)
            + self.pixel_crop_top.size(0x54BB)
            + self.pixel_crop_left.size(0x54CC)
            + self.pixel_crop_right.size(0x54DD)
            + self.display_width.size(0x54B0)
            + self.display_height.size(0x54BA)
            + self.display_unit.size(0x54B2)
            + self.aspect_ratio_type.size(0x54B3)
            + self.colour_space.size(0x2EB524)
            + self.gamma_value.size(0x2FB523)
            + self.frame_rate.size(0x2383E3)
            + self.colour.size(0x55B0)
            + self.projection.size(0x7670)
    }
}

fn gen_track_entry_video<'a>(
    v: &'a Video,
) -> impl Fn((&'a mut [u8], usize)) -> Result<(&'a mut [u8], usize), GenError> {
    move |input| {
        let byte_capacity = vint_size(v.capacity() as u64);

        gen_ebml_master(
            0xE0,
            byte_capacity,
            tuple((
                gen_opt_copy(v.flag_interlaced, |v| gen_ebml_uint(0x9A, v)),
                gen_opt_copy(v.field_order, |v| gen_ebml_uint(0x9D, v)),
                gen_opt_copy(v.stereo_mode, |v| gen_ebml_uint(0x53B8, v)),
                gen_opt_copy(v.alpha_mode, |v| gen_ebml_uint(0x53C0, v)),
                gen_opt_copy(v.old_stereo_mode, |v| gen_ebml_uint(0x53B9, v)),
                gen_ebml_uint(0xB0, v.pixel_width),
                gen_ebml_uint(0xBA, v.pixel_height),
                gen_opt_copy(v.pixel_crop_bottom, |v| gen_ebml_uint(0x54AA, v)),
                gen_opt_copy(v.pixel_crop_top, |v| gen_ebml_uint(0x54BB, v)),
                gen_opt_copy(v.pixel_crop_left, |v| gen_ebml_uint(0x54CC, v)),
                gen_opt_copy(v.pixel_crop_right, |v| gen_ebml_uint(0x54DD, v)),
                gen_opt_copy(v.display_width, |v| gen_ebml_uint(0x54B0, v)),
                gen_opt_copy(v.display_height, |v| gen_ebml_uint(0x54BA, v)),
                gen_opt_copy(v.display_unit, |v| gen_ebml_uint(0x54B2, v)),
                gen_opt_copy(v.aspect_ratio_type, |v| gen_ebml_uint(0x54B3, v)),
                gen_opt(v.colour_space.as_ref(), |v| gen_ebml_binary(0x2EB524, v)),
                gen_opt_copy(v.gamma_value, |v| gen_f64(0x2FB523, v)),
                gen_opt_copy(v.frame_rate, |v| gen_f64(0x2383E3, v)),
                gen_opt(v.colour.as_ref(), gen_track_entry_video_colour),
                gen_opt(v.projection.as_ref(), |v| {
                    gen_track_entry_video_projection(v)
                }),
            )),
        )(input)
    }
}

impl EbmlSize for Colour {
    fn capacity(&self) -> usize {
        self.matrix_coefficients.size(0x55B1)
            + self.bits_per_channel.size(0x55B2)
            + self.chroma_subsampling_horz.size(0x55B3)
            + self.chroma_subsampling_vert.size(0x55B4)
            + self.cb_subsampling_horz.size(0x55B5)
            + self.cb_subsampling_vert.size(0x55B6)
            + self.chroma_siting_horz.size(0x55B7)
            + self.chroma_siting_vert.size(0x55B8)
            + self.range.size(0x55B9)
            + self.transfer_characteristics.size(0x55BA)
            + self.primaries.size(0x55BB)
            + self.max_cll.size(0x55BC)
            + self.max_fall.size(0x55BD)
            + self.mastering_metadata.size(0x55D0)
    }
}

fn gen_track_entry_video_colour<'a>(
    c: &'a Colour,
) -> impl Fn((&'a mut [u8], usize)) -> Result<(&'a mut [u8], usize), GenError> {
    move |input| {
        let byte_capacity = vint_size(c.capacity() as u64);

        gen_ebml_master(
            0x55B0,
            byte_capacity,
            tuple((
                gen_opt_copy(c.matrix_coefficients, |v| gen_ebml_uint(0x55B1, v)),
                gen_opt_copy(c.bits_per_channel, |v| gen_ebml_uint(0x55B2, v)),
                gen_opt_copy(c.chroma_subsampling_horz, |v| gen_ebml_uint(0x55B3, v)),
                gen_opt_copy(c.chroma_subsampling_vert, |v| gen_ebml_uint(0x55B4, v)),
                gen_opt_copy(c.cb_subsampling_horz, |v| gen_ebml_uint(0x55B5, v)),
                gen_opt_copy(c.cb_subsampling_vert, |v| gen_ebml_uint(0x55B6, v)),
                gen_opt_copy(c.chroma_siting_horz, |v| gen_ebml_uint(0x55B7, v)),
                gen_opt_copy(c.chroma_siting_vert, |v| gen_ebml_uint(0x55B8, v)),
                gen_opt_copy(c.range, |v| gen_ebml_uint(0x55B9, v)),
                gen_opt_copy(c.transfer_characteristics, |v| gen_ebml_uint(0x55BA, v)),
                gen_opt_copy(c.primaries, |v| gen_ebml_uint(0x55BB, v)),
                gen_opt_copy(c.max_cll, |v| gen_ebml_uint(0x55BC, v)),
                gen_opt_copy(c.max_fall, |v| gen_ebml_uint(0x55BD, v)),
                gen_opt(c.mastering_metadata.as_ref(), |v| {
                    gen_track_entry_video_colour_mastering_metadata(v)
                }),
            )),
        )(input)
    }
}

impl EbmlSize for MasteringMetadata {
    fn capacity(&self) -> usize {
        self.primary_r_chromaticity_x.size(0x55D1)
            + self.primary_r_chromaticity_y.size(0x55D2)
            + self.primary_g_chromaticity_x.size(0x55D3)
            + self.primary_g_chromaticity_y.size(0x55D4)
            + self.primary_b_chromaticity_x.size(0x55D5)
            + self.primary_b_chromaticity_y.size(0x55D6)
            + self.white_point_chromaticity_x.size(0x55D7)
            + self.white_point_chromaticity_y.size(0x55D8)
            + self.luminance_max.size(0x55D9)
            + self.luminance_min.size(0x55DA)
    }
}

fn gen_track_entry_video_colour_mastering_metadata<'a>(
    m: &'a MasteringMetadata,
) -> impl Fn((&'a mut [u8], usize)) -> Result<(&'a mut [u8], usize), GenError> {
    move |input| {
        let byte_capacity = vint_size(m.capacity() as u64);

        gen_ebml_master(
            0x55D0,
            byte_capacity,
            tuple((
                gen_opt_copy(m.primary_r_chromaticity_x, |v| gen_f64(0x55D1, v)),
                gen_opt_copy(m.primary_r_chromaticity_y, |v| gen_f64(0x55D2, v)),
                gen_opt_copy(m.primary_g_chromaticity_x, |v| gen_f64(0x55D3, v)),
                gen_opt_copy(m.primary_g_chromaticity_y, |v| gen_f64(0x55D4, v)),
                gen_opt_copy(m.primary_b_chromaticity_x, |v| gen_f64(0x55D5, v)),
                gen_opt_copy(m.primary_b_chromaticity_y, |v| gen_f64(0x55D6, v)),
                gen_opt_copy(m.white_point_chromaticity_y, |v| gen_f64(0x55D7, v)),
                gen_opt_copy(m.white_point_chromaticity_y, |v| gen_f64(0x55D8, v)),
                gen_opt_copy(m.luminance_max, |v| gen_f64(0x55D9, v)),
                gen_opt_copy(m.luminance_min, |v| gen_f64(0x55DA, v)),
            )),
        )(input)
    }
}

impl EbmlSize for Projection {
    fn capacity(&self) -> usize {
        self.projection_type.size(0x7671)
            + self.projection_private.size(0x7672)
            + self.projection_pose_yaw.size(0x7673)
            + self.projection_pose_pitch.size(0x7674)
            + self.projection_pose_roll.size(0x7675)
    }
}

fn gen_track_entry_video_projection<'a>(
    p: &'a Projection,
) -> impl Fn((&'a mut [u8], usize)) -> Result<(&'a mut [u8], usize), GenError> {
    move |input| {
        let byte_capacity = vint_size(p.capacity() as u64);

        gen_ebml_master(
            0x7670,
            byte_capacity,
            tuple((
                gen_ebml_uint(0x7671, p.projection_type),
                gen_opt(p.projection_private.as_ref(), |v| {
                    gen_ebml_binary(0x7672, v)
                }),
                gen_f64(0x7673, p.projection_pose_yaw),
                gen_f64(0x7674, p.projection_pose_pitch),
                gen_f64(0x7675, p.projection_pose_roll),
            )),
        )(input)
    }
}

impl<'a> EbmlSize for Cluster<'a> {
    fn capacity(&self) -> usize {
        self.timecode.size(0xE7) + self.silent_tracks.size(0x5854) + self.position.size(0xA7) +
      self.prev_size.size(0xAB) + self.simple_block.size(0xA3) +
      // TODO: implement for BlockGroup
      // self.block_group.size(0xA0) +
      self.encrypted_block.size(0xAF)
    }
}

pub(crate) fn gen_cluster<'a>(
    c: &'a Cluster,
) -> impl Fn((&'a mut [u8], usize)) -> Result<(&'a mut [u8], usize), GenError> {
    move |input| {
        let byte_capacity = vint_size(c.capacity() as u64);
        gen_ebml_master(
            0x1F43B675,
            byte_capacity,
            tuple((
                gen_ebml_uint(0xE7, c.timecode),
                gen_opt(c.silent_tracks.as_ref(), gen_silent_tracks),
                gen_opt_copy(c.position, |v| gen_ebml_uint(0xA7, v)),
                gen_opt_copy(c.prev_size, |v| gen_ebml_uint(0xAB, v)),
                gen_many(&c.simple_block, |v| gen_ebml_binary(0xA3, v)),
                // TODO: implement for BlockGroup
                // gen_many(&c.block_group, gen_block_group)
                gen_opt(c.encrypted_block.as_ref(), |v| gen_ebml_binary(0xAF, v)),
            )),
        )(input)
    }
}

impl EbmlSize for SilentTracks {
    fn capacity(&self) -> usize {
        self.numbers.size(0x58D7)
    }
}

fn gen_silent_tracks<'a>(
    s: &'a SilentTracks,
) -> impl Fn((&'a mut [u8], usize)) -> Result<(&'a mut [u8], usize), GenError> {
    move |input| {
        let byte_capacity = vint_size(s.capacity() as u64);
        gen_ebml_master(
            0x5854,
            byte_capacity,
            gen_many(&s.numbers, |v| gen_ebml_uint(0x58D7, *v)),
        )(input)
    }
}

pub(crate) fn gen_simple_block_header<'a>(
    s: &'a SimpleBlock,
) -> impl Fn((&'a mut [u8], usize)) -> Result<(&'a mut [u8], usize), GenError> {
    move |input| {
        let mut flags = 0u8;

        if s.keyframe {
            flags |= 0b0001u8;
        }

        if s.invisible {
            flags |= 0b00010000u8;
        }

        flags |= match s.lacing {
            Lacing::None => 0u8,
            Lacing::Xiph => 0b00100000u8,
            Lacing::FixedSize => 0b01000000u8,
            Lacing::EBML => 0b01100000u8,
        };

        if s.discardable {
            flags |= 0b10000000u8;
        }

        set_be_u8(
            tuple((gen_vint(s.track_number), |i| set_be_i16(i, s.timecode)))(input)?,
            flags,
        )
    }
}

#[allow(dead_code)]
fn gen_laced_frames<'a>(
    input: (&'a mut [u8], usize),
    lacing: Lacing,
    frames: &[&[u8]],
) -> Result<(&'a mut [u8], usize), GenError> {
    match lacing {
        Lacing::None => Err(GenError::NotYetImplemented),
        Lacing::Xiph => gen_xiph_laced_frames(input, frames),
        Lacing::EBML => gen_ebml_laced_frames(input, frames),
        Lacing::FixedSize => gen_fixed_size_laced_frames(input, frames),
    }
}

#[allow(dead_code)]
fn gen_xiph_laced_frames<'a>(
    _input: (&'a mut [u8], usize),
    frames: &[&[u8]],
) -> Result<(&'a mut [u8], usize), GenError> {
    if frames.is_empty() {
        return Err(GenError::NotYetImplemented);
    }

    Err(GenError::NotYetImplemented)
}

#[allow(dead_code)]
fn gen_ebml_laced_frames<'a>(
    _input: (&'a mut [u8], usize),
    _frames: &[&[u8]],
) -> Result<(&'a mut [u8], usize), GenError> {
    Err(GenError::NotYetImplemented)
}

#[allow(dead_code)]
fn gen_fixed_size_laced_frames<'a>(
    _input: (&'a mut [u8], usize),
    _frames: &[&[u8]],
) -> Result<(&'a mut [u8], usize), GenError> {
    Err(GenError::NotYetImplemented)
}

#[cfg(test)]
mod tests {
    use log::trace;
    use nom::HexDisplay;
    use quickcheck::quickcheck;

    use crate::elements::SegmentElement;

    use super::*;

    fn test_seek_head_serializer(seeks: Vec<(u64, Vec<u8>)>) -> bool {
        trace!("testing for {:?}", seeks);

        let mut should_fail = false;
        if seeks.is_empty() {
            should_fail = true;
        }

        for &(_, ref id) in seeks.iter() {
            trace!("id: {}", id.to_hex(16));
            if id.is_empty() {
                trace!("id is empty, returning");
                return true;
            }
        }

        if should_fail {
            trace!("the parser should fail");
        }

        let capacity = seeks
            .iter()
            .fold(0, |acc, &(_, ref v)| acc + 8 + v.len() + 100);
        trace!("defining capacity as {}", capacity);

        let mut data = vec![0; capacity];

        let seek_head = SeekHead {
            positions: seeks
                .iter()
                .cloned()
                .map(|(position, id)| Seek { id, position })
                .collect(),
        };

        let ser_res = {
            let gen_res = gen_seek_head(&seek_head)((&mut data[..], 0));
            trace!("gen_res: {:?}", gen_res);
            if let Err(e) = gen_res {
                trace!("gen_res is error: {:?}", e);
                trace!("should fail: {:?}", should_fail);
                return should_fail;
            }
        };

        trace!("ser_res: {:?}", ser_res);

        let parse_res = crate::elements::segment_element(&data[..]);
        trace!("parse_res: {:?}", parse_res);
        match parse_res {
            Ok((_rest, SegmentElement::SeekHead(o))) => {
                if should_fail {
                    trace!("parser should have failed on input for {:?}", seek_head);
                    trace!("{}", (&data[..]).to_hex(16));
                    return false;
                }

                assert_eq!(seek_head, o);
                true
            }
            e => {
                if should_fail {
                    return true;
                }

                panic!("{}", format!("parse error: {:?} for input: {:?}", e, seeks))
            }
        }
    }

    quickcheck! {
      fn test_seek_head(seeks: Vec<(u64, Vec<u8>)>) -> bool {
        test_seek_head_serializer(seeks)
      }
    }
}
