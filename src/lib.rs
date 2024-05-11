mod error;

pub use error::Error;
use webm::mux::Track;
use std::{
	io::Cursor, num::NonZeroU64, path::PathBuf
};
use matroska_demuxer::MatroskaFile;

/// This is the name of the WebM clips that the game saves.
pub const FRAGMENT_RECORDING_FILENAME: &str = "output.webm";

/// Concatenate WebM video from an array of byte cursors.
/// 
/// `keep_second_audio_track` determines whether or not to keep the second audio
/// track that is recorded. This track contains only the game audio and no voice
/// is included.
/// 
/// Since there is no way to tell when they were recorded anymore, this function
/// concatenates them in the order they're given. (The date tag is not set by
/// Content Warning, so this isn't usable here.)
pub fn concatenate_from_bytes<R>(bytes_arrays: &mut [R], keep_second_audio_track: bool) -> Result<Vec<u8>, Error>
	where R: std::io::Read + std::io::Seek {
	if bytes_arrays.is_empty() { return Err(Error::NoFiles) }

	let output = Vec::with_capacity(4_000_000); // 4 MB
	// Keep track of where we are in the stream
	let mut timestamp_offset: u64 = 0;
	let mut timestamp_scale: u64;
	let writer = webm::mux::Writer::new(Cursor::new(output));
	let mut video_id = unsafe { NonZeroU64::new_unchecked(1) };
	// This is a hack, I couldn't find a similar codec ID
	// in matroska_demuxer
	let video_codec = webm::mux::VideoCodecId::VP8;
	let mut video_width = 1;
	let mut video_height = 1;

	let mut audio0_id = unsafe { NonZeroU64::new_unchecked(2) };
	// This is a hack, I couldn't find a similar codec ID
	// in matroska_demuxer
	let audio0_codec = webm::mux::AudioCodecId::Vorbis;
	let mut audio0_sample_rate = 0;
	let mut audio0_channels = 0;

	let mut audio1_id = unsafe { NonZeroU64::new_unchecked(3) };
	// This is a hack, I couldn't find a similar codec ID
	// in matroska_demuxer
	let audio1_codec = webm::mux::AudioCodecId::Vorbis;
	let mut audio1_sample_rate = 0;
	let mut audio1_channels = 0;
	let mut segment = webm::mux::Segment::new(writer).unwrap();

	segment.set_app_name("libcwcat 0.1.0");

	let first_video_start_pos = bytes_arrays[0].stream_position()?;
	// Get the video track details
	let first_video = MatroskaFile::open(&mut bytes_arrays[0])?;

	let mut audio0_found = false;
	for track in first_video.tracks() {
		if track.flag_enabled() {
			if let Some(video_track) = track.video() {
				video_width = video_track.pixel_width().into();
				video_height = video_track.pixel_height().into()
			}
			if let Some(audio_track) = track.audio() {
				if audio0_found {
					audio1_sample_rate = audio_track.sampling_frequency() as i32;
					audio1_channels = audio_track.channels().into();
					break
				} else {
					audio0_sample_rate = audio_track.sampling_frequency() as i32;
					audio0_channels = audio_track.channels().into()
				}
				audio0_found = true
			}
		}
	}

	drop(first_video);
	bytes_arrays[0].seek(std::io::SeekFrom::Start(first_video_start_pos))?;

	let (audio_track0_id, video_track_id, audio_track1_id) = (1, 2, 3);

	let mut audio_track0 = segment.add_audio_track(audio0_sample_rate, audio0_channels as i32, Some(audio_track0_id), audio0_codec);

	let mut video_track = segment.add_video_track(video_width as u32, video_height as u32, Some(video_track_id), video_codec);

	let mut audio_track1 = if keep_second_audio_track { Some(
		segment.add_audio_track(audio1_sample_rate, audio1_channels as i32, Some(audio_track1_id), audio1_codec)
	)} else { None };

	// This is a hack, I think?
	let mut added_video_meta = false;

	for bytes in bytes_arrays {
		let mut file = MatroskaFile::open(bytes)?;
		timestamp_scale = file.info().timestamp_scale().into();

		let mut audio0_found = false;
		// Some sanity checks to confirm the files aren't changing, but we
		// also need to get the new track IDs while we're here
		for track in file.tracks() {
			if track.flag_enabled() {
				if let Some(video) = track.video() {
					if video_width != video.pixel_width().into() || video_height != video.pixel_height().into() {
						return Err(Error::VideoResolutionChanges {
							old_w: video_width, old_h: video_width, new_w: video.pixel_width().into(), new_h: video.pixel_height().into()
						})
					}
					if !added_video_meta {
						let bit_depth = video.colour().map(|c| c.bits_per_channel().unwrap_or(10)).unwrap_or(10) as u8;
						let subsampling_h = video.colour().map(|c| c.chroma_sitting_horz()) == Some(Some(matroska_demuxer::ChromaSitingHorz::Unknown));
						let subsampling_v = video.colour().map(|c| c.chroma_sitting_vert()) == Some(Some(matroska_demuxer::ChromaSitingVert::Unknown));
						let full_range = video.colour().map(|c| c.range()) == Some(Some(matroska_demuxer::Range::Full));
						if !video_track.set_color(bit_depth, (!subsampling_h, !subsampling_v), full_range) {
							return Err(Error::SetColorError)
						};

						if let Some(data) = track.codec_private() {
							// Why is the ID suddenly a u64??????
							if !segment.set_codec_private(video_track_id as u64, data) {
								return Err(Error::SetPrivateDataError(video_track_id))
							}
						}

						added_video_meta = true
					}

					video_id = track.track_number()
				}
				if let Some(audio_track) = track.audio() {
					if audio0_found {
						if audio1_sample_rate != audio_track.sampling_frequency() as i32 {
							return Err(Error::SampleRateChanges { old: audio1_sample_rate, new: audio_track.sampling_frequency() as i32 })
						}
						if audio1_channels != audio_track.channels().into() {
							return Err(Error::ChannelChanges { old: audio1_channels, new: audio_track.channels().into() })
						}
						audio1_id = track.track_number();

						if keep_second_audio_track {
							if let Some(data) = track.codec_private() {
								// Why is the ID suddenly a u64??????
								if !segment.set_codec_private(audio_track1_id as u64, data) {
									return Err(Error::SetPrivateDataError(audio_track1_id))
								}
							}
						}

						break
					} else {
						if audio0_sample_rate != audio_track.sampling_frequency() as i32 {
							return Err(Error::SampleRateChanges { old: audio0_sample_rate, new: audio_track.sampling_frequency() as i32 })
						}
						if audio0_channels != audio_track.channels().into() {
							return Err(Error::ChannelChanges { old: audio0_channels, new: audio_track.channels().into() })
						}
						audio0_id = track.track_number();

						if let Some(data) = track.codec_private() {
							// Why is the ID suddenly a u64??????
							if !segment.set_codec_private(audio_track0_id as u64, data) {
								return Err(Error::SetPrivateDataError(audio_track0_id))
							}
						}
					}
					audio0_found = true
				}
			}
		}

		let mut frame = matroska_demuxer::Frame::default();
		let mut last_timestamp = 0;
		loop {
			if !file.next_frame(&mut frame)? { break }
			let time = (frame.timestamp * timestamp_scale) + timestamp_offset;

			match frame.track {
				i if i == audio0_id.into() => {
					if !audio_track0.add_frame(&frame.data, time, frame.is_keyframe.unwrap()) {
						return Err(Error::AddFrameError { timestamp: time, track_id: audio_track0_id, size: frame.data.len() })
					};
					last_timestamp = time
				},
				i if i == video_id.into() => {
					if !video_track.add_frame(&frame.data, time, frame.is_keyframe.unwrap()) {
						return Err(Error::AddFrameError { timestamp: time, track_id: video_track_id, size: frame.data.len() })
					};
					last_timestamp = time
				},
				i if keep_second_audio_track && i == audio1_id.into() => {
					if !audio_track1.as_mut().unwrap().add_frame(&frame.data, time, frame.is_keyframe.unwrap()) {
						return Err(Error::AddFrameError { timestamp: time, track_id: audio_track1_id, size: frame.data.len() })
					};
					last_timestamp = time
				}
				_ => {}
			}
		}

		timestamp_offset = last_timestamp
	}

	let writer = match segment.try_finalize(Some(timestamp_offset / 1_000_000)) {
		Ok(writer) => writer,
		Err(_) => return Err(Error::FinalizeError)
	};

	Ok(writer.unwrap().into_inner())
}

/// Returns a Vec of all clip fragment directories, sorted from oldest to newest
/// creation date.
/// 
/// This function checks for the existence of files with the name of
/// [FRAGMENT_RECORDING_FILENAME] inside of subdirectories of the given path,
/// and returns a Vec containing only those directories that met this condition.
/// 
/// For a directory that doesn't seem to have any recordings (like an empty
/// directory) this function will return an empty Vec.
/// 
/// If an accurate creation date is not available, this function will not be
/// reliable and it might not be possible to concatenate the clips except by
/// hand.
pub fn list_from_rec_path<P>(path: P) -> Result<Vec<(PathBuf, std::fs::Metadata)>, Error>
	where P: Into<PathBuf> {
	let rec_path = path.into();

	// Find all the directories that have the clips we want
	let mut dirs: Vec<(PathBuf, std::fs::Metadata, std::time::SystemTime)> = Vec::new();

	for entry in std::fs::read_dir(rec_path)? {
		let entry = entry?;
		let metadata = entry.metadata()?;
		if metadata.is_dir() {
			if entry.path().join(FRAGMENT_RECORDING_FILENAME).exists() {
				let created = metadata.created()?;
				dirs.push((entry.path(), metadata, created))
			}
		}
	}

	// Sort them by date created
	dirs.sort_by(|a, b| {
		a.2.cmp(&b.2)
	});
	
	// Remove system time
	let mut dirs_out = Vec::with_capacity(dirs.len());

	for (path, metadata, _) in dirs {
		dirs_out.push((path, metadata))
	}

	Ok(dirs_out)
}

/// Concatenate WebM video from the path to a rec directory.
/// 
/// `keep_second_audio_track` determines whether or not to keep the second audio
/// track that is recorded. This track contains only the game audio and no voice
/// is included.
/// 
/// For a directory that doesn't seem to have any recordings (like an empty
/// directory) this function will return an error.
/// 
/// If an accurate creation date is not available, this function will not be
/// reliable and it might not be possible to concatenate the clips except by
/// hand.
pub fn concatenate_from_rec_path<P>(path: P, keep_second_audio_track: bool) -> Result<Vec<u8>, Error>
	where P: Into<PathBuf> {

	// Open these files
	let mut bytes_arrays = Vec::new();

	for (fragment, _) in list_from_rec_path(path.into())? {
		bytes_arrays.push(
			std::fs::File::open(fragment.join(FRAGMENT_RECORDING_FILENAME))?
		)
	}

	// And concatenate them
	concatenate_from_bytes(&mut bytes_arrays, keep_second_audio_track)
}
