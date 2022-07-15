#version 430

in vec3 position;
in vec3 normal;
in vec2 texcoord0;

out vec4 v_model_pos;
out vec4 v_view_pos;
out vec4 v_perspective_pos;
out vec3 v_normal;
out vec2 v_uv;

uniform int vertex_count;
uniform vec4 resolution;
uniform mat4 model_matrix;
uniform mat3 normal_matrix;
uniform float time;

const float PI = acos( -1.0 );

mat2 rot2d( float t ) {
    return mat2( cos( t ), sin( t ), -sin( t ), cos( t ) );
}

mat4 lookAtInverse( vec3 pos, vec3 tar, vec3 up, float roll ) {
    vec3 dir = normalize( pos - tar );
    vec3 sid = normalize( cross( up, dir ) );
    vec3 top = cross( dir, sid );
    sid = sid * cos( roll ) + top * sin( roll );
    top = cross( dir, sid );

    return mat4(
        sid.x, top.x, dir.x, 0.0,
        sid.y, top.y, dir.y, 0.0,
        sid.z, top.z, dir.z, 0.0,
        -dot( sid, pos ),
        -dot( top, pos ),
        -dot( dir, pos ),
        1.0
    );
}

mat4 perspective( float fov, float near, float far ) {
    float p = 1.0 / tan( fov * PI / 360.0 );
    float d = ( far - near );
    return mat4(
        p, 0.0, 0.0, 0.0,
        0.0, p, 0.0, 0.0,
        0.0, 0.0, -( far + near ) / d, -1.0,
        0.0, 0.0, -2 * far * near / d, 0.0
    );
}

void main() {
    float aspect = resolution.x / resolution.y;

    mat4 view_matrix = lookAtInverse(
        vec3( 0.0, 0.0, 5.0 ),
        vec3( 0.0, 0.0, 0.0 ),
        vec3( 0.0, 1.0, 0.0 ),
        0.0
    );
    mat4 perspective_matrix = perspective( 45.0, 0.01, 20.0 );

    v_model_pos = vec4( 100.0 * position, 1.0 ); // BoomBox.glbくん、キミ小さすぎない？？？ glTFサンプルモデルの自覚ある？？？
    v_model_pos = model_matrix * v_model_pos;
    v_model_pos.zx = rot2d( time ) * v_model_pos.zx;
    v_view_pos = view_matrix * v_model_pos;
    v_perspective_pos = perspective_matrix * v_view_pos;
    v_perspective_pos.x /= aspect;

    gl_Position = v_perspective_pos;

    v_normal = normalize( normal_matrix * normal );
    v_normal.zx = rot2d( time ) * v_normal.zx;

    v_uv = texcoord0;
}
