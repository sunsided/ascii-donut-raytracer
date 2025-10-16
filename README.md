# ASCII Donut Raytracer (Rust Port)

This project is a **Rust port** of the C++ torus raytracer described in the article  
[“How to draw donut using C++ and raytracing”](https://medium.com/@idimus/how-to-draw-donut-using-c-and-raytracing-c07778f45952).  
It renders a rotating torus (donut) using signed distance fields (SDFs) and simple ray marching, displayed directly in the terminal as ASCII art.

## Overview

The program simulates a 3D torus illuminated by a directional light source and viewed through a simple pinhole camera model.  
Each frame:

1. Casts a ray per character cell.
2. Marches the ray until it intersects the torus surface.
3. Computes a normal vector numerically.
4. Uses the dot product with the light direction for diffuse shading.
5. Maps brightness to a character from a gradient to form the image.

The result is an animated, shaded donut rotating in 3D space.

## Features

- Fully written in **Rust**, using only the standard library and [`crossterm`](https://crates.io/crates/crossterm) for terminal rendering.  
- Real-time ASCII rendering at typical console frame rates.  
- Simple vector math implementation for portability.  
- Diffuse lighting model using surface normals from SDF gradients.  
- Adjustable screen resolution, light vector, and radii.

## Building

```bash
git clone https://github.com/sunsided/ascii-donut-raytracer.git
cd ascii-donut-raytracer
cargo run --release
```

## Configuration

You can tweak key parameters inside `main()`:

| Parameter       | Description                            | Default |
|-----------------|----------------------------------------|----------|
| `in_rad`        | Tube radius of the torus               | 0.3      |
| `out_rad`       | Main radius of the torus               | 1.2      |
| `light`         | Direction of incoming light            | (-1,-1,-1) |
| `ro`            | Camera position                        | (-2.5,0,0) |
| `moving`        | Number of animation frames             | 20000    |
| `pixel_aspect`  | Correction for terminal character ratio | 11/24    |

## How It Works

### Signed Distance Function (SDF)

The torus surface is defined as a function returning the shortest distance from any point in space to the surface:

```
sdTorus(p, t, tdir) = length(projected(p) - p) - tube_radius
```

This allows the renderer to march rays efficiently toward the surface.

### Surface Normal

The normal vector is approximated numerically using finite differences:

```
n = normalize(vec3(
    sd(p+εx) - sd(p-εx),
    sd(p+εy) - sd(p-εy),
    sd(p+εz) - sd(p-εz)
))
```

### Lighting

A single diffuse light source contributes brightness based on the dot product between the surface normal and the light direction.


## License

MIT License.
Original C++ concept and algorithm by [Idimus](https://medium.com/@idimus).
