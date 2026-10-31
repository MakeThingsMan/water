struct VertexOut {
  @builtin(position) position: vec4f,
  @location(0) color: vec3f,
}

@vertex
fn vertex(@builtin(vertex_index) vertexIndex: u32) -> VertexOut {
  var positions = array<vec2f, 3>(
    vec2f(0.0, 0.5),
    vec2f(-0.5, -0.5),
    vec2f(0.5, -0.5),
  );

  var colors = array<vec3f, 3>(
    vec3f(1.0, 0.0, 0.0),
    vec3f(0.0, 1.0, 0.0),
    vec3f(0.0, 0.0, 1.0),
  );

  var out: VertexOut;
  out.position = vec4f(positions[vertexIndex], 0.0, 1.0);
  out.color = colors[vertexIndex];
  return out;
}


@fragment
fn fragment(@location(0) color: vec3f) -> @location(0) vec4f {
    return vec4f(color,1.0); 
}
