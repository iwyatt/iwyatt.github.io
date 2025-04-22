#!/usr/bin/env nu

def main [] {
  for file in (ls *.gif | where type == file) {
    let filename = ($file.name | str replace ".gif" ".png")
    ffmpeg -i $file.name -frames:v 1 -vf "colorkey=0x29ab82:0.01:0.0" $filename
    print $"Converted ($file.name) to PNG frames."
  }
  print "Conversion complete."
}
