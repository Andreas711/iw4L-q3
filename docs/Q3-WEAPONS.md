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


## Live developer controls

The live runtime keeps a separate Q3 weapon inventory and ammo pool per client, but weapon
selection now uses the normal `give` command.

```
give shotgun
give rocket
give railgun
```

Giving a Q3 weapon grants its original Quake III weapon-pickup ammo quantity and immediately puts
that weapon in the player's hands. Repeating `give <q3 weapon>` grants the pickup ammo again and
switches back to it.

Q3 aliases are accepted directly, including `gauntlet`, `machinegun`, `shotgun`, `grenade`,
`rocket`, `lightning`, `railgun`, `plasma`, and `bfg`. The explicit forms
`give q3/<weapon>` and `give q3:<weapon>` are also accepted.

The old `q3give` and `q3use` commands have been removed. To return to an IW weapon, use the
normal IW give syntax, for example `give weapon/<game:weapon>`; doing so disables Q3 weapon
handling before equipping the IW weapon.

`q3fire <weapon>` remains a direct one-shot debug path for isolated combat testing.


## Q3 damage movement semantics

Q3 projectile explosions now use the base-game damage/knockback ordering from `g_combat.c`:
knockback is calculated from the pre-self-damage amount, while self-inflicted health damage is then
halved. With the base `g_knockback=1000` and player mass 200, a 100-damage rocket can contribute
500 units/s of velocity along the normalised blast direction before movement processing. Radius
knockback also uses Q3's +24 Z bias, which is what makes rocket jumps lift the player.

A projectile direct-hit target is excluded from that projectile's splash pass, matching
`G_MissileImpact` calling `G_RadiusDamage(..., ignore=other, ...)`. This prevents a direct rocket,
plasma bolt, grenade or BFG hit from accidentally receiving both direct and splash damage.

Railgun now follows Q3's multi-target trace behaviour for up to four damageable player hits before
a solid world hit terminates the beam.

Q3 weapon ownership, selected weapon, ammo and next-fire deadline are included in authoritative
client snapshot metadata, so prediction/replay snapshot adoption no longer silently loses the Q3
weapon runtime.


## Native Quake III first-person models

The Q3 runtime can now load the original weapon MD3s directly from a user-owned Quake III
`baseq3/pak0.pk3`. No Q3 game assets are committed to this repository.

Set either:

```
IW4L_Q3_BASEQ3=C:/Games/Quake III Arena/baseq3
```

or point the variable directly at `pak0.pk3`. If `IW4L_Q3_BASEQ3` is omitted, the loader also
checks a few conventional Q3 directory names below `IW4L_GAMES`.

At startup the asset lane reads the nine base weapon models and the normal Q3 companion
`_hand.md3`, `_barrel.md3` and `_flash.md3` models when present. The first-person presentation
uses the hand model's `tag_weapon` transform and replaces the normal IW4 FPV geometry with the
selected Q3 MD3 while keeping IW4L's existing viewmodel placement/render pipeline.

Use:

```
q3assets
```

to verify the PK3 was found and see how many Q3 weapon models were loaded.

The first visual milestone deliberately reuses the currently prepared IW4 material technique while
feeding it Q3 geometry. This gets the real Q3 weapon shape into the first-person render path without
redistributing id's game data. Q3 shader/texture resolution and native Q3 weapon sounds are the next
presentation layer; gameplay remains supplied by `weapon_q3`.
