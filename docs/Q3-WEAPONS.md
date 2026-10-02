# Quake III weapon port

This branch implements Quake III Arena weapon behaviour as native Rust rules inside IW4L.

The behavioural references are the official id Software Quake III Arena source tree:

- `code/game/g_weapon.c` — gauntlet, machinegun, shotgun, lightning and railgun firing behaviour
- `code/game/g_missile.c` — grenade, rocket, plasma and BFG projectile values
- `code/game/bg_public.h` — shared constants such as shotgun pellet count/spread and lightning range
- `code/game/bg_pmove.c` — weapon timing / refire behaviour

The original code is GPL-licensed. IW4L does not copy the C implementation. The Rust code here is an
independent implementation of the observable mechanics and numeric gameplay parameters.

## Base Q3 values represented in weapon_q3

| Weapon | Core behaviour |
| --- | --- |
| Gauntlet | 50 damage, 32-unit trace |
| Machinegun | 7 damage, spread 200 |
| Shotgun | 11 pellets, 10 damage each, spread 700 |
| Grenade Launcher | 100 direct / 100 splash, radius 150, speed 700, 2.5 s fuse |
| Rocket Launcher | 100 direct / 100 splash, radius 120, speed 900 |
| Lightning Gun | 8 damage per pulse, range 768 |
| Railgun | 100 damage, range 8192 |
| Plasma Gun | 20 direct / 15 splash, radius 20, speed 2000 |
| BFG10K | 100 direct / 100 splash, radius 120, speed 2000 |

Integration policy: IW4L remains authoritative for collision, lag compensation, snapshots, damage,
death handling, replay and networking. `weapon_q3` supplies the Q3 rules.
