struct Light {
  position: vec3<f32>,
  color: vec3<f32>,
}

// struct CameraUniform {
//   view_proj: mat4x4<f32>,
// };

// @group(1) @binding(0)
// var<uniform> camera: CameraUniform;

@group(2) @binding(0)
var<uniform> light: Light;

struct Camera { 
  view_pos: vec4f,
  view_proj: mat4x4<f32>,
}

@group(1) @binding(0)
var<uniform> camera: Camera;

struct InstanceInput {
  @location(5) model_matrix_0: vec4<f32>,
  @location(6) model_matrix_1: vec4<f32>,
  @location(7) model_matrix_2: vec4<f32>,
  @location(8) model_matrix_3: vec4<f32>,
  @location(9) normal_matrix_0: vec3<f32>,
  @location(10) normal_matrix_1: vec3<f32>,
  @location(11) normal_matrix_2: vec3<f32>,
};

struct VertexInput {
  @location(0) position: vec3<f32>,
  @location(1) tex_coords: vec2<f32>,
  @location(2) normal: vec3<f32>,
};

struct VertexOut {
  @builtin(position) clip_position: vec4<f32>,
  // @location(0) color: vec3f,
  @location(0) tex_coords: vec2<f32>,
  @location(1) world_normal: vec3<f32>,
  @location(2) world_position: vec3<f32>,
};

@vertex
fn vs_main (
  model: VertexInput,
  instance: InstanceInput,
) -> VertexOut{//(@builtin(vertex_index) vertexIndex: u32) -> VertexOut {
  // var positions = array<vec2f, 6>(
  //   vec2f(-0.5,  0.5),
  //   vec2f(-0.5, -0.5),
  //   vec2f( 0.5, -0.5),
  //   vec2f( -0.5, 0.5),
  //   vec2f( 0.5, 0.5),
  //   vec2f( 0.5, -0.5),
  // );

  // var colors = array<vec3f, 6>(
  //   vec3f(1.0, 0.0, 0.0),
  //   vec3f(0.0, 1.0, 0.0),
  //   vec3f(0.0, 0.0, 1.0),
  //   vec3f(0.0, 0.0, 1.0),
  //   vec3f(0.0, 1.0, 0.0),
  //   vec3f(1.0, 0.0, 0.0),
  // );

  // var out: VertexOut;
  // out.position = vec4f(positions[vertexIndex], 0.0, 1.0);
  // out.color = colors[vertexIndex];
  // return out;

  let model_matrix = mat4x4<f32>( // we set this before the camera matrix changes everything because these are in world space coordinates and the camera matrix transposes everything into camera space. 
    instance.model_matrix_0,
    instance.model_matrix_1,
    instance.model_matrix_2,
    instance.model_matrix_3,
  );

  let normal_matrix = mat3x3<f32> (
    instance.normal_matrix_0,
    instance.normal_matrix_1,
    instance.normal_matrix_2,
  );
//TODO: When making stuff for the game we're going to need to convert all this from world space to view space. 
  var out: VertexOut;
  out.tex_coords = model.tex_coords;
  out.world_normal = normal_matrix * model.normal;
  var world_position: vec4<f32>  = model_matrix * vec4<f32>(model.position, 1.0);
  out.world_position = world_position.xyz;
  out.clip_position = camera.view_proj * world_position;
  return out;
}

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) //binding describes where its slotted into in the sampler in this instance.
var s_diffuse: sampler;


@fragment
//fn fragment(@location(0) color: vec3f) -> @location(0) vec4f {
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
  let object_color: vec4<f32> = textureSample(t_diffuse, s_diffuse, in.tex_coords);
  
  // We don't need (or want) much ambient light, so 0.1 is fine
  let ambient_strength = 0.1;
  let ambient_color = light.color * ambient_strength;
  
  let light_dir = normalize(light.position - in.world_position);

  let view_dir = normalize(camera.view_pos.xyz - in.world_position);
  let half_dir  = normalize(view_dir + light_dir);

  // let reflect_dir = reflect(-light_dir, in.world_normal);
  // let specular_strength = pow(max(dot(view_dir, reflect_dir), 0.0), 16.0); // the old Blinn-Phong model had an issue when the angle between the viewer and the reflection surpassed 90 degrees
  let specular_strength = pow(max(dot(in.world_normal, half_dir),0.0), 16.0);
  let specular_color = specular_strength * light.color;

  // my assumption is that we can clamp this down to be the various bands of color that are present within cell shading. 
  let diffuse_strength = max(dot(in.world_normal, light_dir), 0.0); 
  let diffuse_color = light.color * diffuse_strength;

  let result = (ambient_color + diffuse_color + specular_color) * object_color.rgb;

  return vec4f(result, object_color.a);
  //  return vec4f(color,1.0); 
}
