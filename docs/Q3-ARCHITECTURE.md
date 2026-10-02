# Quake III weapon subsystem architecture

The Quake III weapons are a parallel weapon domain inside IW4L, not aliases or
skins for MW2 weapons.

## Shared engine services

Q3 is allowed to reuse engine-level services that are not weapon rules:

- player input and authoritative ticks
- player/world collision and lag-compensated trace geometry
- entity allocation/lifetime
- snapshot transport and replay transport
- generic damage delivery to player health
- camera transforms
- low-level GPU upload/render scheduling

These are IW4L engine services, not MW2 weapon definitions.

## Q3-owned systems

Q3 owns all weapon-specific state and behaviour:

- inventory and selected weapon
- ammo
- fire timing and weapon switching
- spread/pellet generation
- hitscan range and piercing rules
- projectile speed, trajectory, bounce and lifetime
- direct/splash damage
- self-damage and knockback
- muzzle/impact/projectile events
- first-person weapon model
- projectile model
- Q3 shader/texture resolution
- weapon audio and effects

Q3 code must not look up an IW4 weapon merely to obtain gameplay or presentation
behaviour.

## Identity rule

An IW4 weapon id and a Q3 weapon id are different domains.

Q3 hitscan damage uses the Q3 weapon identity and does not require a carrier
IW4 weapon. Q3 projectiles use `ProjectileState::q3_weapon`; their legacy
`weapon` field is zero until the common projectile container is replaced by a
native Q3 projectile container.

## Presentation domain

`render_q3` is a separate first-person presentation lane. It converts MD3
surfaces directly to Bevy meshes and binds Q3 textures directly through
Q3-owned materials. It shares the engine's existing world camera/render target
instead of creating a second Camera3d, because IW4's custom render graph is
defined around one world view. It does not mutate IW4 viewmodel geometry,
borrow an MW2 weapon material, or hijack an MW2 texture slot.

When a Q3 weapon is active, the IW4 first-person draw plan is simply left empty.
The two presentation domains therefore do not share weapon models or materials;
they only meet at the generic engine camera/render target boundary.

Projectile presentation is the next part of this same boundary: Rocket,
Grenade, Plasma and BFG visuals should be submitted by `render_q3` from the
Q3 projectile identity rather than from an IW4 weapon definition.

The normal `give` console command is the development entry point for both
domains. Bare Q3 weapon names grant and immediately select a Q3 weapon;
`give weapon/<game:weapon>` returns to the IW domain. The console transport
mechanism is not part of Q3 gameplay semantics.

## Target flow

```
UserCmd
  -> Q3 weapon runtime
  -> Q3 fire decision
  -> Q3 hitscan OR Q3 projectile
  -> shared world collision/trace
  -> Q3 damage + knockback
  -> shared health/death authority
  -> Q3 snapshot/presentation
```

MW2 remains on its existing path:

```
UserCmd
  -> weapon_iw4
  -> IW4 fire/projectile logic
  -> shared world authority
```

The two weapon domains should meet only at generic engine boundaries.
