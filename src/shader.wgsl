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
fn vertex (model: VertexInput,) -> VertexOut{//(@builtin(vertex_index) vertexIndex: u32) -> VertexOut {
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

  var out: VertexOut;
  out.tex_coords = model.tex_coords;
  out.clip_position = vec4<f32>(model.position,1.0);
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
