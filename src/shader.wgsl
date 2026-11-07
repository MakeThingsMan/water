struct CameraUniform {
  view_proj: mat4x4<f32>,
};

@group(1) @binding(0)
var<uniform> camera: CameraUniform;

struct InstanceInput {
  @location(5) model_matrix_0: vec4<f32>,
  @location(6) model_matrix_1: vec4<f32>,
  @location(7) model_matrix_2: vec4<f32>,
  @location(8) model_matrix_3: vec4<f32>,
};

struct VertexInput {
  @location(0) position: vec3<f32>,
  @location(1) tex_coords: vec2<f32>,
};

struct VertexOut {
  @builtin(position) clip_position: vec4<f32>,
  // @location(0) color: vec3f,
  @location(0) tex_coords: vec2<f32>,
};

@vertex
fn vertex (
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

  var out: VertexOut;
  out.tex_coords = model.tex_coords;
  out.clip_position = camera.view_proj * model_matrix * vec4<f32>(model.position,1.0);
  return out;
}

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) //binding describes where its slotted into in the sampler in this instance.
var s_diffuse: sampler;

@fragment
//fn fragment(@location(0) color: vec3f) -> @location(0) vec4f {
fn fragment(in: VertexOut) -> @location(0) vec4<f32> {
  return textureSample(t_diffuse, s_diffuse, in.tex_coords);
  //  return vec4f(color,1.0); 
}
