struct Light {
  position: vec3<f32>,
  color: vec3<f32>,
}

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
  @location(0) position:    vec3<f32>,
  @location(1) tex_coords:  vec2<f32>,
  @location(2) normal:      vec3<f32>,
  @location(3) tangent:     vec3<f32>,
  @location(4) bitangent:   vec3<f32>,
};

struct VertexOut {
  @builtin(position) clip_position: vec4<f32>,
  @location(0) tex_coords: vec2<f32>,
  @location(1) tangent_position: vec3<f32>,
  @location(2) tangent_light_position: vec3<f32>,
  @location(3) tangent_view_position: vec3<f32>,
};


@vertex
fn vs_main (
  model: VertexInput,
  instance: InstanceInput,
) -> VertexOut{
  
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
  
  let coordinate_system = mat3x3<f32>(
    vec3(1,0,0), // x-axis (right)
    vec3(0,1,0), // y-axis (up)
    vec3(0,0,1), // z-axis (forward)
  );


  // Cosntruct the tangent matrix
  let world_normal = normalize(normal_matrix * model.normal);
  let world_tangent= normalize(normal_matrix * model.tangent);
  let world_bitangent = normalize(normal_matrix * model.bitangent);
  let tangent_matrix = transpose(mat3x3<f32>(
    world_tangent,
    world_bitangent,
    world_normal,
  ));

  let world_position = model_matrix * vec4f(model.position, 1.0);
  
  //TODO: When making stuff for the game we're going to need to convert all this from world space to view space. I think this would do good for the optimization where you unload the things that aren't in the direct view of the camera as well.  
  var out: VertexOut;
  out.clip_position = camera.view_proj * world_position;
  out.tex_coords = model.tex_coords;
  out.tangent_position = tangent_matrix * world_position.xyz;
  out.tangent_view_position = tangent_matrix * camera.view_pos.xyz;
  out.tangent_light_position= tangent_matrix * light.position; 
  return out;
}

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) //binding describes where its slotted into in the sampler in this instance.
var s_diffuse: sampler;
@group(0) @binding(2)
var t_normal: texture_2d<f32>;
@group(0) @binding(3)
var s_normal: sampler;

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
  let object_color: vec4<f32> =  textureSample(t_diffuse,s_diffuse, in.tex_coords);
  let object_normal: vec4<f32> = textureSample(t_normal, s_normal,  in.tex_coords);
  
  // We don't need (or want) much ambient light, so 0.1 is fine
  let ambient_strength = 0.1;
  let ambient_color = light.color * ambient_strength;
  
  let tangent_normal = object_normal.xyz * 2.0 - 1.0;
  
  let light_dir = normalize(in.tangent_light_position - in.tangent_position);

  let view_dir = normalize(in.tangent_view_position - in.tangent_position);
  // let half_dir  = normalize(view_dir + light_dir);

  // let reflect_dir = reflect(-light_dir, in.world_normal);
  // let specular_strength = pow(max(dot(view_dir, reflect_dir), 0.0), 16.0); // the old Blinn-Phong model had an issue when the angle between the viewer and the reflection surpassed 90 degrees
  let specular_strength = pow(max(dot(tangent_normal, light_dir),0.0), 16.0); 
  let specular_color = specular_strength * light.color;

  // my assumption is that we can clamp this down to be the various bands of color that are present within cell shading. 
  let diffuse_strength = pow(max(dot(tangent_normal, light_dir), 0.0), 16.0); 
  let diffuse_color = light.color * diffuse_strength;

  let result = (ambient_color + diffuse_color + specular_color) * object_color.rgb;

  return vec4f(result, object_color.a);
}