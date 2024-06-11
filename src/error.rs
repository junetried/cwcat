pub use matroska_demuxer::DemuxError;

/// Errors that can be returned from this library.
#[derive(Debug)]
pub enum Error {
	/// matroska_demuxer returned an error.
	DemuxError (DemuxError),
	/// Failed to set color information.
	/// 
	/// This is a libwebm error.
	SetColorError,
	/// Failed to set private codec data.
	/// 
	/// This is a libwebm error.
	SetPrivateDataError (i32),
	/// Failed to add a frame.
	/// 
	/// This is a libwebm error.
	AddFrameError { timestamp: u64, track_id: i32, size: usize },
	/// Failed to finalize the file.
	/// 
	/// This is a libwebm error.
	FinalizeError,
	/// There were no files given.
	NoFiles,
	/// The number of audio channels change.
	ChannelChanges { old: u64, new: u64 },
	/// The sample rate of audio changes.
	SampleRateChanges { old: i32, new: i32 },
	/// The resolution of video changes.
	VideoResolutionChanges { old_w: u64, old_h: u64, new_w: u64, new_h: u64 },
	/// Couldn't determine if a frame was a keyframe or not.
	UnknownKeyframe,
	/// Duration info was missing.
	MissingDuration,
	/// IO error.
	IOError(std::io::Error)
}

impl From<DemuxError> for Error {
	fn from(value: DemuxError) -> Self {
		Error::DemuxError(value)
	}
}

impl From<std::io::Error> for Error {
	fn from(value: std::io::Error) -> Self {
		Error::IOError(value)
	}
}

impl std::fmt::Display for Error {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::DemuxError (demux_error) => write!(f, "Demuxing error: {}", demux_error),
			Self::SetColorError => write!(f, "Couldn't set color data for libwebm video track"),
			Self::SetPrivateDataError (track_id) => write!(f, "Couldn't set private codec data for track {track_id}"),
			Self::AddFrameError { timestamp, track_id, size } => write!(f, "Couldn't add frame of size {size} bytes to track {track_id} at timestamp {timestamp}"),
			Self::FinalizeError => write!(f, "libwebm couldn't finalize"),
			Self::NoFiles => write!(f, "There were no files to concatenate"),
			Self::ChannelChanges { old, new } => write!(f, "The number of channels changes between files (old: {old}, new: {new})"),
			Self::SampleRateChanges { old, new } => write!(f, "The sample rate changes between files (old: {old}, new: {new})"),
			Self::VideoResolutionChanges { old_w, old_h, new_w, new_h } => write!(f, "The video resolution changes between files (old: {old_w}x{old_h}, new: {new_w}x{new_h})"),
			Self::UnknownKeyframe => write!(f, "is_keyframe returned None"),
			Self::MissingDuration => write!(f, "Duration in info was not specified"),
			Self::IOError (io_error) => write!(f, "IO error: {}", io_error),
		}
	}
}
