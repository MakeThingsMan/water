@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) 
var s_diffuse: sampler;

struct FragmentInput {
   @builtin(position) clip_position: vec4<f32>,
}

const PI = 3.14159265358979323846;

@vertex
fn vs_main(@builtin(vertex_index) vertexIndex: u32) -> FragmentInput {
    let positions = array<vec2f, 3>(
        vec2f(-1.0,  -1.0),
        vec2f(4.0, -1.0), // 3 looked like it worked visually but I fear that might not hold if the aspect ratio of the screen changes at all. 
        vec2f(-1.0, 4.0),
    );

    var out: FragmentInput;

    out.clip_position = vec4f(positions[vertexIndex], 0.0, 1.0);
    return out; 
}

fn sum(in: vec3f) -> f32{
    return in.r + in.g + in.b;
}

fn quadrant(center: vec2<i32>, dir: vec2<i32>) -> vec4<f32>{
    var color_sum = vec3f(0.0);
    var brightness_sum = 0.0; 
    var brightness_sq_sum = 0.0; 
    let kernelRadius = 5; 
    let size = vec2<i32>(textureDimensions(t_diffuse));

    for(var x: i32 = 0; x <= kernelRadius; x++) {
        for(var y: i32 = 0; y <= kernelRadius; y++) {
            let pixel = clamp(center + vec2<i32>(x * dir.x,y*dir.y),vec2<i32>(0,0),size - 1);
            let color = textureLoad(t_diffuse,pixel,0).rgb;
            let brightness = sum(color.rgb)/3;
            color_sum += color;
            brightness_sum += brightness;
            brightness_sq_sum += brightness * brightness;
        }
    }

    let n = f32((kernelRadius + 1) * (kernelRadius + 1));
    let mean = color_sum/n;
    let variance = brightness_sq_sum / n - pow((brightness_sum / n),2);
    var out: vec4<f32>;
    out.x = mean.x;
    out.y = mean.y;
    out.z = mean.z;
    out.w = variance;
    return out;
}   

@fragment
fn fs_main(in: FragmentInput) -> @location(0) vec4<f32> {
    let center = vec2<i32>(in.clip_position.xy);
    let main_texture = textureLoad(t_diffuse, center, 0);

    let direction_matrix = array<vec2<i32>, 4> (
        vec2<i32>(-1,1),
        vec2<i32>(-1,-1),
        vec2<i32>(1,-1),
        vec2<i32>(1,1),
    );

    let temp  = quadrant(center,direction_matrix[0]);
    let temp1 = quadrant(center,direction_matrix[1]);
    let temp2 = quadrant(center,direction_matrix[2]);
    let temp3 = quadrant(center,direction_matrix[3]);

    var out:vec4f = temp;

    out = select(out, temp1, temp1.w < out.w);
    out = select(out, temp2, temp2.w < out.w);
    out = select(out, temp3, temp3.w < out.w);

    return vec4f(out.xyz, 1.0);
}   