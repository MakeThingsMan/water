struct VertexInput {
  @location(0) position: vec3<f32>,
  @location(1) color: vec3<f32>,
};

struct VertexOut {
  @builtin(position) clip_position: vec4f,
  @location(0) color: vec3f,
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
  out.color = model.color;
  out.clip_position = vec4<f32>(model.position,1.0);
  return out;
}


@fragment
fn fragment(@location(0) color: vec3f) -> @location(0) vec4f {
    return vec4f(color,1.0); 
}
