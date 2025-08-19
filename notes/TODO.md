
TODO List:

# Gameplay
- [X] Prefab spawning, primarily for items currently
- [ ] Generate items inside of a digsite.
- [ ] Surround digsite in fences with some sides open, add colliders for fences as well.
- [X] Voxel falling physics, just simple `if air below, move down` would be good enough, can think about falling sand esque stuff later.
    - [X] Sand
    - [X] Liquids
    - Optimizations
        - [ ] Multithreading?
        - [ ] Remove update indices, operate on chunks
        - [ ] 64-tree
        - [ ] Margolus neighborhoods?
- [X] Camera decision, top down or fps? (FPS decided)
- [ ] Digsite
    - [ ] Obstacles?
        - [ ] Stones/boulders
        - [ ] Mines/explosives?
        - [ ] ~liquids? This seems hard~
        - [ ] Permit zones (can't use certain tools, or vehicles, adds some puzzle element to a digsite)
        - [ ] ?
- [ ] Transport
    - [ ] Road generation from digsite to processing area, roads maybe increase walking/driving speed
    - [ ] Vehicles
- [ ] Processing area
    - [ ] Fossil cleaning
    - [ ] Ore refinery
    - [ ] ?

# Rendering
- [ ] Create Object ID texture map for use in the edge detection to avoid the depth map steepness issues.
- [ ] Fade out shader for voxels and other objects in the way of the player.