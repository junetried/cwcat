# cwcat
A library that can concatenate videos from the game
[Content Warning](https://store.steampowered.com/app/2881650/Content_Warning/).

This uses the [matroska-demuxer](https://crates.io/crates/matroska-demuxer)
and [webm](https://crates.io/crates/webm) crates to concatenate videos that the
game saves. The intended use of this is to recover videos that were not saved
due to player death, the game crashing, or negligence.

Since this library does
no encoding or decoding, it should be very fast, even with many clips. It should
also work with mods that change the allowed video length or bitrate or
resolution. The only assumptions are that the videos are VP8 and the audio is
Vorbis. Things about the video and audio, like sample rate, frame rate, and
channel count, must not change between clips; these are checked and the library
will return an error if it finds any of them have changed.

It only has a few functions to use, and they should be easy to understand how to
use if you read the documenting comments.

## Second audio track?
The game saves its clips with two audio tracks: the first is the one you
normally hear in extracted video. The second is a little louder and only
contains game audio. This audio track is probably used to make the final track,
but otherwise I'm not sure why it's there.

There are some other goodies, too. In the clip directories, you can find two
extra files: `audio.raw` and `mic.raw`. These are both raw audio data:
32-bit float, little-endian, with two channels, and 48KHz and 24 KHz
respectively. In theory, these could be mixed to create a higher quality audio
track, since the game mixes them into a 24KHz track. I haven't decided if that's
out of scope for this library or not. It sounds fun though.

## Where does the game save my videos?
It uses a temporary directory, which it deletes on game exit. If you're using
Windows, you can open it by pressing F3.

Videos that have been extracted will contain a `fullRecording.webm` file and an
accompanying text file that lists the path to all the clips in order, in the
format of
[FFMpeg's concat format option](https://ffmpeg.org/ffmpeg-formats.html#concat).

A video that hasn't been extracted yet will not have either of these files, but
will still have all of the clips.

## How does the game concatenate videos?
The output files that it saves report libavformat as the muxer, so probably by
invoking FFMpeg. This would explain part of why the game freezes for so
long (relatively) when extracting video, and would explain the bug cited on
[the game's FAQ](https://landfall.se/content-warning-faq) where it apparently
breaks if an apostraphe is in the directory path. It would also explain the text
file video list.

## Why not just do it like the game does?
That would be too simple. This way, it's more fun.

Oh, and also, the input text file isn't actually generated until the game
attempts to extract the video, so it's not always going to be an option anyway.