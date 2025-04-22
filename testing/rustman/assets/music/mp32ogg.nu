#!/usr/bin/env nu

def main [] {
  for file in (ls *.mp3 | where type == file) {
    let filename = ($file.name | str replace ".mp3" ".ogg")
    ffmpeg -i $file.name $filename
    print $"Converted ($file.name) to ($filename)"
  }
  print "Conversion complete."
}
