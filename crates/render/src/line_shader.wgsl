// Line shader: each instance is one line; six vertices make its quad.

struct Viewport {
    size_pixels: vec2<f32>,
    // Uniform buffers are sized in multiples of 16 bytes.
    padding: vec2<f32>,
}

@group(0) @binding(0) var<uniform> viewport: Viewport;

struct Line {
    @location(0) start_end_pixels: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) thickness_pixels: f32,
}

struct Varying {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

// Two triangles: (along the line 0..1, across the line -1..1).
var<private> CORNERS: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
    vec2<f32>(0.0, -1.0), vec2<f32>(1.0, -1.0), vec2<f32>(0.0, 1.0),
    vec2<f32>(0.0, 1.0), vec2<f32>(1.0, -1.0), vec2<f32>(1.0, 1.0),
);

@vertex
fn vertex_main(@builtin(vertex_index) corner_index: u32, line: Line) -> Varying {
    let start: vec2<f32> = line.start_end_pixels.xy;
    let end: vec2<f32> = line.start_end_pixels.zw;
    let along: vec2<f32> = end - start;
    let length_pixels: f32 = length(along);
    // A zero-length line has no direction; it is drawn as a square dot
    // along x, which is harmless.
    var direction: vec2<f32> = vec2<f32>(1.0, 0.0);
    if (length_pixels > 0.0) {
        direction = along / length_pixels;
    }
    let normal: vec2<f32> = vec2<f32>(-direction.y, direction.x);
    let corner: vec2<f32> = CORNERS[corner_index];
    let pixel: vec2<f32> = mix(start, end, corner.x)
        + normal * corner.y * (line.thickness_pixels * 0.5);
    // Pixels (origin top-left, y down) to clip space (origin centre, y up).
    let clip: vec2<f32> = vec2<f32>(
        pixel.x / viewport.size_pixels.x * 2.0 - 1.0,
        1.0 - pixel.y / viewport.size_pixels.y * 2.0,
    );
    var out: Varying;
    out.position = vec4<f32>(clip, 0.0, 1.0);
    out.color = line.color;
    return out;
}

@fragment
fn fragment_main(in: Varying) -> @location(0) vec4<f32> {
    return in.color;
}
