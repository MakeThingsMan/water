@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

struct FragmentInput {
    @builtin(position) clip_position: vec4<f32>,
}

const PI = 3.14159265358979323846;

struct Tunables {
    alpha: f32,
    kernel_radius: u32,
    zeta: f32,
    zerocross: f32,
    sectors:u32,
    hardness: f32,
    q: f32,
}

@group(0) @binding(2)
var<uniform> tunables: Tunables; //var<uniform> because it reads from a uniform buffer!!!!

@vertex
fn vs_main(@builtin(vertex_index) vertexIndex: u32) -> FragmentInput {
    let positions = array<vec2f, 3>(
            vec2f(-1.0, -1.0),
            vec2f(
                4.0,
                -1.0,
            ),
            vec2f(-1.0, 4.0),
        );

    var out: FragmentInput;

    out.clip_position = vec4f(positions[vertexIndex], 0.0, 1.0);
    return out;
}

fn gaussian(sigma: f32, pos: f32) -> f32 {
    return (1.0 / sqrt(2.0 * PI * sigma * sigma)) * exp(-(pos * pos) / (2.0 * sigma * sigma));
}

fn sum(in: vec3f) -> f32 {
    return in.r + in.g + in.b;
}

@fragment
fn fs_main(in: FragmentInput) -> @location(0) vec4<f32> {
    let kernelRadius = tunables.kernel_radius/2;
    let center = vec2<i32>(in.clip_position.xy);
    let main_texture = textureLoad(t_diffuse, center, 0);
    let main_texture_texel_size = vec2<i32>(textureDimensions(t_diffuse));

    let a = f32(kernelRadius) * clamp(((tunables.alpha + 0.0) / tunables.alpha), 0.1, 2.0);
    let b = f32(kernelRadius) * clamp((tunables.alpha / (tunables.alpha + 0.0)), 0.1, 2.0);

    let cos_phi = cos(0.0);
    let sin_phi = sin(0.0);

    let R: mat2x2f = mat2x2f(
            vec2f(cos_phi, -sin_phi),
            vec2f(sin_phi, cos_phi),
        );

    let S: mat2x2f = mat2x2f(
            vec2f(0.5 / a, 0.0),
            vec2f(0.0, 0.5 / b),
        );

    let SR = S * R;

    let max_x = i32(sqrt(a * a * cos_phi * cos_phi + b * b * sin_phi * sin_phi));
    let max_y = i32(sqrt(a * a * sin_phi * sin_phi + b * b * cos_phi * cos_phi));

    let sin_zero_cross = sin(tunables.zerocross);
    let eta = ((tunables.zeta + cos(tunables.zerocross)) / (sin_zero_cross * sin_zero_cross));

    var m_array: array<vec4f, 8>;
    var s_array: array<vec3f, 8>;

    for(var i: u32; i < tunables.sectors; i++) {
        m_array[i] = vec4f(0.0);
        s_array[i] = vec3f(0.0);
    }

    for(var y: i32 = -max_y; y <= max_y; y++) {
        for(var x: i32 = -max_x; x <= max_x; x++) {
            let xy_vector = vec2<f32>(vec2<i32>(x, y)); // some odd syntax but it works ig
            var circle = SR * xy_vector; 
            if dot(circle, circle) < .25 {  // the dreaded if statement!!!!!
                var c = textureLoad(
                        t_diffuse,
                        center + vec2<i32>(xy_vector),
                        0,
                    );
                c = saturate(c);
                var sum: f32 = 0.0;
                var w_array: array<f32, 8>;
                var z: f32;
                var vxx: f32;
                var vyy: f32;

                // still haven't the slightest clue what all this is doing but it's here now!
                vxx = tunables.zeta - eta * circle.x * circle.x;
                vyy = tunables.zeta - eta * circle.y * circle.y;
                z = max(0, circle.y + vxx);
                w_array[0] = z * z;
                sum += w_array[0];
                z = max(0, -circle.x + vyy);
                w_array[2] = z * z;
                sum += w_array[2];
                z = max(0, -circle.y + vxx);
                w_array[4] = z * z;
                sum += w_array[4];
                z = max(0, circle.x + vyy);
                w_array[6] = z * z;
                sum += w_array[6];
                circle = sqrt(2.0) / 2.0 * vec2f(circle.x - circle.y, circle.x + circle.y);
                vxx = tunables.zeta - eta * circle.x * circle.x;
                vyy = tunables.zeta - eta * circle.y * circle.y;
                z = max(0, circle.y + vxx);
                w_array[1] = z * z;
                sum += w_array[1];
                z = max(0, -circle.x + vyy);
                w_array[3] = z * z;
                sum += w_array[3];
                z = max(0, -circle.y + vxx);
                w_array[5] = z * z;
                sum += w_array[5];
                z = max(0, circle.x + vyy);
                w_array[7] = z * z;
                sum += w_array[7];

                let g = exp(-3.125 * dot(circle,circle)) / sum;

                for(var i: u32 = 0; i < tunables.sectors; i++) {
                    let thing = w_array[i] * g;
                    m_array[i] += vec4f(c.xyz * thing, thing); 
                    s_array[i] += c.xyz * c.xyz * thing; 
                }
            }
        }
    }

    // let pixel_angle = atan2(in.clip_position.y, in.clip_position.x); // where does this even get used???
    var out =vec4f();

    for (var i: u32 = 0; i < tunables.sectors; i++) {
        m_array[i].r /= m_array[i].w;
        m_array[i].g /= m_array[i].w;
        m_array[i].b /= m_array[i].w;
        s_array[i] = abs(s_array[i] / m_array[i].w - m_array[i].rgb * m_array[i].rgb);

        let sigma2 = s_array[i].r + s_array[i].g + s_array[i].b;
        let w = 1.0 / (1.0 + pow(tunables.hardness * 1000.0 * sigma2, 0.5 * tunables.q));

        out += vec4f(m_array[i].rgb * w, w);
    }

    return saturate(out / out.w);
}
