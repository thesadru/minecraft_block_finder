# Minecraft Block Finder

Finds specific blocks in a minecraft world by parsing `.mca` region files.

## How to use
```
minecraft_block_finder.exe --help
```

Search for diamond ore in the world called `MyWorld`:
```
minecraft_block_finder.exe "diamond_ore" --path C:/Users/admin/AppData/Roaming/.minecraft/saves/MyWorld/region
```

## Config file
The program can also use values in a `config.toml` placed in the current directory.
```toml
block = "diamond_ore" # block to search for
path = "C:/Users/admin/AppData/Roaming/.minecraft/saves/MyWorld/region"  # path to the default world
home = [4000, -3000] # when printing results, first print the chunks closest to this location
show_all = true # whether to show all blocks rather than only chunks containing them
max_distance = 1000 # search only at most this many blocks far
```
