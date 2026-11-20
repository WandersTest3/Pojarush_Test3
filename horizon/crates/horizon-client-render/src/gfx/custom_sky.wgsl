struct Layer {
    clip_from_sky: mat4x4<f32>,
    color: vec4<f32>,
}
@group(0) @binding(0) var<uniform> layer: Layer;
@group(0) @binding(1) var sky_texture: texture_2d<f32>;
@group(0) @binding(2) var sky_sampler: sampler;

struct Out {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex fn vs_custom_sky(@location(0) position: vec3<f32>, @location(1) uv: vec2<f32>) -> Out {
    var out: Out;
    out.position = layer.clip_from_sky * vec4(position, 1.0);
    out.uv = uv;
    return out;
}

@fragment fn fs_custom_sky(in: Out) -> @location(0) vec4<f32> {
    return textureSample(sky_texture, sky_sampler, in.uv) * layer.color;
}
