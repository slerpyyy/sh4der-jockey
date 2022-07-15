#version 430

in vec4 v_model_pos;
in vec4 v_view_pos;
in vec3 v_normal;
in vec2 v_uv;

out vec4 out_color;

uniform vec4 resolution;
uniform float time;

uniform float material_roughness;
uniform float material_metallic;
uniform float material_occlusion_texture_strength;
uniform float material_normal_texture_scale;
uniform vec3 material_emissive;
uniform vec4 material_base_color;
uniform sampler2D material_base_texture;
uniform sampler2D material_metallic_roughness_texture;
uniform sampler2D material_normal_texture;
uniform sampler2D material_emissive_texture;
uniform sampler2D material_occlusion_texture;

const float PI = acos( -1.0 );

// Yoinked from Three.js (MIT)
// https://github.com/mrdoob/three.js/blob/master/LICENSE
vec3 perturb_normal( vec3 view_pos, vec3 normal, vec3 normal_map, float face_dir ) {
    vec3 q0 = vec3( dFdx( view_pos.x ), dFdx( view_pos.y ), dFdx( view_pos.z ) );
    vec3 q1 = vec3( dFdy( view_pos.x ), dFdy( view_pos.y ), dFdy( view_pos.z ) );
    vec2 st0 = dFdx( v_uv.st );
    vec2 st1 = dFdy( v_uv.st );

    vec3 N = normal;

    vec3 q1perp = cross( q1, N );
    vec3 q0perp = cross( N, q0 );

    vec3 T = q1perp * st0.x + q0perp * st1.x;
    vec3 B = q1perp * st0.y + q0perp * st1.y;

    float det = max( dot( T, T ), dot( B, B ) );
    float scale = ( det == 0.0 ) ? 0.0 : face_dir * inversesqrt( det );
    scale *= material_normal_texture_scale;

    return normalize( T * ( normal_map.x * scale ) + B * ( normal_map.y * scale ) + N * normal_map.z );
}

vec3 fresnel_schlick( float VdotH, vec3 f0, vec3 f90 ) {
    float fresnel = pow( max( 0.0, 1.0 - VdotH ), 5.0 );
    return mix( f0, f90, fresnel );
}

float v_ggx( float NdotL, float NdotV, float roughnessSq ) {
    float GGXV = NdotL * sqrt( NdotV * NdotV * ( 1.0 - roughnessSq ) + roughnessSq );
    float GGXL = NdotV * sqrt( NdotL * NdotL * ( 1.0 - roughnessSq ) + roughnessSq );

    float GGX = GGXV + GGXL;
    return ( 0.0 < GGX ) ? ( 0.5 / GGX ) : 0.0;
}

float d_ggx( float NdotH, float roughnessSq ) {
    float f = ( NdotH * NdotH ) * ( roughnessSq - 1.0 ) + 1.0;
    return roughnessSq / ( PI * f * f );
}

vec3 brdf_ggx( vec3 L, vec3 V, vec3 N, vec3 base_color, float roughness, float metallic ) {
    vec3 H = normalize( L + V );
    float NdotL = max( dot( N, L ), 0.0 );
    float NdotV = max( dot( N, V ), 0.0 );
    float NdotH = max( dot( N, H ), 0.0 );
    float VdotH = max( dot( V, H ), 0.0 );

    float roughnessSq = roughness * roughness;

    vec3 f0 = mix( vec3( 0.04 ), base_color, metallic );
    vec3 f90 = vec3( 1.0 );

    vec3 F = fresnel_schlick( VdotH, f0, f90 );
    float Vis = v_ggx( NdotL, NdotV, roughnessSq );
    float D = d_ggx( NdotH, roughnessSq );

    return F * Vis * D;
}

void main() {
    float face_dir = gl_FrontFacing ? 1.0 : -1.0;
    vec3 L = normalize( vec3( 0.0, 1.0, 0.0 ) );

    vec4 base_color = material_base_color;
    base_color *= vec4( pow( texture( material_base_texture, v_uv ).rgb, vec3( 2.2 ) ), 1.0 );

    float roughness = material_roughness;
    float metallic = material_metallic;

    vec4 roughness_metallic_tex = texture( material_metallic_roughness_texture, v_uv );
    roughness *= roughness_metallic_tex.g;
    metallic *= roughness_metallic_tex.b;

    vec3 normal = normalize( v_normal );
    vec3 normal_tex = texture( material_normal_texture, v_uv ).xyz * 2.0 - 1.0;
    normal = perturb_normal( v_view_pos.xyz, normal, normal_tex, face_dir );

    vec3 emissive = material_emissive;
    emissive *= pow( texture( material_emissive_texture, v_uv ).rgb, vec3( 2.2 ) );

    float ao = 1.0;
    ao = mix( material_occlusion_texture_strength, 1.0, texture( material_occlusion_texture, v_uv ).r );

    // lighting
    float NdotL = max( dot( L, normal ), 0.0 );
    vec3 irradiance = 4.0 * vec3( 0.8, 0.82, 0.9 ) * NdotL;
    vec3 diffuse = irradiance * base_color.rgb / PI;
    vec3 specular = irradiance * brdf_ggx( L, -normalize( v_view_pos.xyz ), normal, base_color.rgb, roughness, metallic );

    // ambient
    diffuse += base_color.rgb * vec3( 0.04, 0.06, 0.09 );

    out_color = vec4( ao * ( diffuse + specular ) + emissive, 1.0 );
    out_color.rgb = pow( out_color.rgb, vec3( 0.4545 ) );
}
