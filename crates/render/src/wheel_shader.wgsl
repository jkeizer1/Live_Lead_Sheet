// Wheel shader: one quad over the wheel; every pixel works out its own wedge,
// the gaps between wedges, and the current chord's fill and outline, so no
// geometry is built. Only the current chord stands out: brightened and
// outlined, the other wedges dimmed (EFS-1.10).

struct Wedge {
    color: vec4<f32>,
    // x: 0 idle, 1 current (brightened and outlined)
    mode: vec4<f32>,
}

struct Wheel {
    // width and height of the window in pixels
    viewport_pixels: vec4<f32>,
    // centre x, centre y, inner radius, outer radius, all in pixels
    geometry_pixels: vec4<f32>,
    wedges: array<Wedge, 7>,
}

@group(0) @binding(0) var<uniform> wheel: Wheel;

const PI: f32 = 3.14159265;
const WEDGE_ANGLE_RADIANS: f32 = 0.8975979; // 2π / 7
const GAP_HALF_FRACTION_OF_OUTER: f32 = 0.006;
const OUTLINE_FRACTION_OF_OUTER: f32 = 0.014;
const IDLE_ALPHA: f32 = 0.14;
// The current wedge: its tint lifted this far towards white, so it stands
// out even when every wedge shares one tint (colour off), and its outline
// lifted further.
const CURRENT_LIFT: f32 = 0.25;
const OUTLINE_LIFT: f32 = 0.5;
var<private> CORNERS: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
    vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0), vec2<f32>(-1.0, 1.0),
    vec2<f32>(-1.0, 1.0), vec2<f32>(1.0, -1.0), vec2<f32>(1.0, 1.0),
);

struct Varying {
    @builtin(position) position: vec4<f32>,
    @location(0) pixel: vec2<f32>,
}

@vertex
fn vertex_main(@builtin(vertex_index) corner_index: u32) -> Varying {
    let center: vec2<f32> = wheel.geometry_pixels.xy;
    // A margin of 2 pixels keeps the antialiased edge inside the quad.
    let half_extent: f32 = wheel.geometry_pixels.w + 2.0;
    let pixel: vec2<f32> = center + CORNERS[corner_index] * half_extent;
    let clip: vec2<f32> = vec2<f32>(
        pixel.x / wheel.viewport_pixels.x * 2.0 - 1.0,
        1.0 - pixel.y / wheel.viewport_pixels.y * 2.0,
    );
    var out: Varying;
    out.position = vec4<f32>(clip, 0.0, 1.0);
    out.pixel = pixel;
    return out;
}

@fragment
fn fragment_main(in: Varying) -> @location(0) vec4<f32> {
    let offset: vec2<f32> = in.pixel - wheel.geometry_pixels.xy;
    let inner: f32 = wheel.geometry_pixels.z;
    let outer: f32 = wheel.geometry_pixels.w;
    let radius: f32 = length(offset);

    // Angle clockwise from straight up, then turned so wedge 0 is centred
    // at the top: its edges are at -π/7 and +π/7.
    var angle: f32 = atan2(offset.x, -offset.y) + WEDGE_ANGLE_RADIANS * 0.5;
    if (angle < 0.0) {
        angle = angle + 2.0 * PI;
    }
    let index: u32 = min(u32(floor(angle / WEDGE_ANGLE_RADIANS)), 6u);
    let from_center: f32 = angle - (f32(index) + 0.5) * WEDGE_ANGLE_RADIANS;

    // Distance in pixels from this pixel to the nearest edge of its wedge.
    let to_side: f32 = radius * (WEDGE_ANGLE_RADIANS * 0.5 - abs(from_center));
    let to_ring: f32 = min(radius - inner, outer - radius);
    let to_edge: f32 = min(to_side, to_ring);

    let gap_half: f32 = outer * GAP_HALF_FRACTION_OF_OUTER;
    let outline_width: f32 = outer * OUTLINE_FRACTION_OF_OUTER;
    // Coverage 0..1, antialiased over one pixel.
    let inside: f32 = clamp(to_edge - gap_half + 0.5, 0.0, 1.0);
    let in_outline_band: f32 = clamp(gap_half + outline_width + 0.5 - to_edge, 0.0, 1.0);

    let wedge: Wedge = wheel.wedges[index];
    if (wedge.mode.x > 0.5) {
        // Current: a lifted fill, opaque, with a lighter outline over its
        // edge.
        let fill_color: vec3<f32> = mix(wedge.color.rgb, vec3<f32>(1.0), CURRENT_LIFT);
        let outline_color: vec3<f32> = mix(wedge.color.rgb, vec3<f32>(1.0), OUTLINE_LIFT);
        let color: vec3<f32> = mix(fill_color, outline_color, in_outline_band);
        return vec4<f32>(color, inside);
    }
    return vec4<f32>(wedge.color.rgb, inside * IDLE_ALPHA);
}
