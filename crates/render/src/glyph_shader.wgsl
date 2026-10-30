// Glyph shader: each instance is one glyph; six vertices make its quad. The
// atlas holds signed distance fields, so the edge is drawn sharp at any size.

struct Viewport {
    size_pixels: vec2<f32>,
    // Uniform buffers are sized in multiples of 16 bytes.
    padding: vec2<f32>,
}

@group(0) @binding(0) var<uniform> viewport: Viewport;
@group(0) @binding(1) var atlas_texture: texture_2d<f32>;
@group(0) @binding(2) var atlas_sampler: sampler;

struct Glyph {
    // left, top, width, height in window pixels
    @location(0) rect_pixels: vec4<f32>,
    // left, top, right, bottom in the atlas, 0 to 1
    @location(1) atlas_rect: vec4<f32>,
    @location(2) color: vec4<f32>,
    // screen pixels per unit of field value
    @location(3) distance_scale_pixels: f32,
}

struct Varying {
    @builtin(position) position: vec4<f32>,
    @location(0) atlas_position: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) distance_scale_pixels: f32,
}

// Two triangles over the unit square, (0, 0) top-left.
var<private> CORNERS: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
    vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
    vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0),
);

@vertex
fn vertex_main(@builtin(vertex_index) corner_index: u32, glyph: Glyph) -> Varying {
    let corner: vec2<f32> = CORNERS[corner_index];
    let pixel: vec2<f32> = glyph.rect_pixels.xy + corner * glyph.rect_pixels.zw;
    // Pixels (origin top-left, y down) to clip space (origin centre, y up).
    let clip: vec2<f32> = vec2<f32>(
        pixel.x / viewport.size_pixels.x * 2.0 - 1.0,
        1.0 - pixel.y / viewport.size_pixels.y * 2.0,
    );
    var out: Varying;
    out.position = vec4<f32>(clip, 0.0, 1.0);
    out.atlas_position = mix(glyph.atlas_rect.xy, glyph.atlas_rect.zw, corner);
    out.color = glyph.color;
    out.distance_scale_pixels = glyph.distance_scale_pixels;
    return out;
}

@fragment
fn fragment_main(in: Varying) -> @location(0) vec4<f32> {
    // 0.5 is the outline; above is inside.
    let value: f32 = textureSample(atlas_texture, atlas_sampler, in.atlas_position).r;
    let distance_pixels: f32 = (value - 0.5) * in.distance_scale_pixels;
    // Coverage over one screen pixel across the edge: the antialiasing.
    let coverage: f32 = clamp(distance_pixels + 0.5, 0.0, 1.0);
    return vec4<f32>(in.color.rgb, in.color.a * coverage);
}
