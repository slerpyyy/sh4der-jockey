# Sh4der Jockey
*シェーダーを使ったVJツール by sp4ghet & slerpy*

## とりあえず動かす

バイナリをどこかにインストールしておきます。追加でバイナリのあるディレクトリにUIの設定等を保存するのでWrite権限のある場所に置いてください。
将来的に他にもファイルが書かれる可能性があります。

作業ディレクトリでバイナリを`-i`フラグで実行するとExampleプロジェクトが生成されます。

## UI

ボタンやスライダーに対してMIDIを割り当てることが可能です。`bind`を長押ししながらMIDIコントローラーを操作して、最後に受信したMIDIキーと結び付けられます。

## Config File
A config file is a special yaml file called `config.yaml` at the project root alongside the pipeline files (described below).
An example config file is shown below:

コンフィグファイルはプロジェクトディレクトリの直下にある `config.yaml` という名前の特殊な設定ファイルで、pipelineを跨いだ設定項目をいくつか設定できます。設定されていない場合はデフォルトですべてのMIDIデバイスとデフォルトオーディオ入力デバイスに接続を試みます。

```yaml
midi_devices:
  - "My MIDI Substr"
  - "Other Device"
audio_device: "Audio Input Substr"
```

この設定はオーディオ入力デバイスとMIDIデバイス名の部分文字列とマッチするか、で検索します。

## パイプライン

プログラムは起動したときに`cwd`直下にある`.yaml`ファイルを探してパイプライン(Pipeline)ファイルとして扱います。
複数のパイプラインファイルが見つかった場合、コントロールパネル（UIウィンドウ）からどのパイプラインを実行するか選択できます。

パイプラインファイルの例が以下にあります。
他にもExampleプロジェクトのパイプラインファイル等を見てください。

基本的な構造としてステージ(stage)がいくつかあり、それが上から順に実行されていきます。
ステージは `fragment`, `vertex`, `vertex+fragment`, `compute`の種類があります。
```yaml
stages:
  - cs: "particle_pos.comp"
    target: "particle_pos"
    resolution: [10000, 200, 2]
    dispatch_size: [100, 200, 1]

  - vs: "draw_particle.vert"
    count: 8000000
    mode: LINES
    target: particles
    point_size: 2

  - fs: "scenes/scene_a.glsl"
    target: "render"
    float: true
    mipmap: true

  - fs: "post_process.frag"
    uniforms:
      - chromab: 0.4

ndi:
  - source: "source substring"
    name: "sampler_name"

images:
  - path: "images/image.png"
    name: "some_image"

audio:
  audio_samples: 8192
  spectrum:
    mipmap: true
    filter: linear
    wrap_mode: repeat
```

## Fragment シェーダー

```glsl
#version 440

out vec4 out_color;

uniform vec4 resolution;
uniform float time;

void main() {
    vec2 uv = gl_FragCoord.xy / resolution.xy;
    out_color = vec4(uv, 0.5 + 0.5 * sin(time), 1);
}
```

### 必須項目

フラグメントステージは必ず以下の項目が設定されている必要があります。

 - `fs: Path` フラグメントシェーダーのglslファイルへの相対パス。

### 任意項目

 - `target: String` レンダーターゲットの名前。シェーダーで使うuniformとして使われます。
   - default: 画面に描画されるフレームバッファへ直接書き出されます。
 - `resolution: [Int; 2]` targetの解像度。
   - default: ウィンドウの解像度
 - `wrap: {clamp, repeat}` targetのテキスチャラッピングモード
   - default: clamp
 - `filter: {linear, nearest}` targetのテキスチャダウン/アップサンプリングモード
   - default: linear
   - `mipmap`が`true`の場合 `MIPMAP_X`に設定されます。
 - `mipmap: Bool` targetに対するmipmapを有効化するか
   - default: false
 - `float: Bool` float textureを有効化するか
   - default: false


### フラグメントステージ固有のVarying

```glsl
// ピクセルの色。頂点ステージでfsが指定されていない場合はv_colorが出力されます。
out vec4 out_color;
```

## 頂点シェーダー

```glsl
#version 140

out vec4 v_color;
uniform int vertex_count;

void main(){
  float a = 12 * gl_VertexID / vertex_count;
  gl_VertexPos = vec4(cos(a), sin(a), a, 1);

  v_color = vec4(1);
}
```

### 必須項目

頂点ステージは必ず以下の項目が設定されている必要があります。

 - `vs: Path` 頂点シェーダーのglslファイルへの相対パス。

### 任意項目

 - `count: Int` 出力される頂点数
    - default: 2000
 - `mode: {LINE_LOOP, LINE_STRIP, LINES, POINTS, TRIANGLE_FAN, TRIANGLE_STRIP, TRIANGLES}`
    - default: POINTS
    - OpenGLの描画モードとしてそのまま適用されます。
 - `thickness: Float` 頂点や線の太さをピクセル単位で指定します。
    - default: 1
    - この項目はGPUによっては使い物にならない場合があります。
 - `fs: Path` 適用するフラグメントシェーダーへの相対パス
    - default: デフォルトのフラグメントシェーダーを適用します。
 - `target: String` レンダーターゲットの名前。シェーダーで使うuniformとして使われます。
    - default: 画面に描画されるフレームバッファへ直接書き出されます。
 - `resolution: [Int; 2]` targetの解像度。
   - default: ウィンドウの解像度
 - `wrap: {clamp, repeat}` targetのテキスチャラッピングモード
   - default: clamp
 - `filter: {linear, nearest}` targetのテキスチャダウン/アップサンプリングモード
   - default: linear
   - `mipmap`が`true`の場合 `MIPMAP_X`に設定されます。
 - `mipmap: Bool` targetに対するmipmapを有効化するか
   - default: false
 - `float: Bool` float textureを有効化するか
   - default: false


### 頂点ステージ固有のvaryingやuniform

```glsl
// countで指定された頂点数。
uniform int vertex_count;

// 点, 線, ポリゴンの頂点カラー
out vec4 v_color;

// 使うと多分セグフォして死にます
// 触らぬ神に祟りなし。
in vec2 position;
```

## Computeシェーダー

```glsl
#version 430

layout(local_size_x = 2, local_size_y = 2) in;
layout(rgba32f) uniform image2D img_output;

uniform vec4 resolution;

void main() {
  // GlobalInvocationIDからピクセル座標を計算
  ivec2 pixel_coords = ivec2(gl_GlobalInvocationID.xy);
  vec4 pixel = imageLoad(img_output, pixel_coords);
  pixel.rg = pixel_coords / resolution.xy;


  // 特定の座標に出力
  imageStore(img_output, pixel_coords, pixel);
}
```

テキスチャ全体に対してプログラムが実行されるためには以下の条件が満たされている必要があります。
`local_size_(xyz) * dispatch.(xyz) == resolution.(xyz)`

### 必須項目

コンピュートステージでは以下の項目が設定されている必要があります。

 - `cs: Path` コンピュートシェーダーのglslファイルへの相対パス。
 - `dispatch: [Int; 1-3]` dispatchの数
 - `resolution: [Int; 1-3]` targetの解像度
 - `target: String` targetの名前。
   - 追記: コンピュートステージでは`imageND`のターゲットが生成されます。 `samplerND`として使えますが, mipmap等は有効化できません。

## 画像

```yaml
images:
   - path: "relative/to/cwd.png"
     name: "name_of_uniform_sampler_2D"
   - path: "second/image/path"
     name: "uniform_of_second_image"
```

```glsl
uniform sampler2D {name_of_image};
uniform vec4 {name_of_image}_res; // vec4(x, y, z, x/y)
```

現状静止画しかサポートしていません. `png` と `jpeg`は検証しましたが他の画像でも動くかもしれません.

## オーディオ設定

```yaml
audio:
  audio_samples: int
  spectrum:
    mipmap: bool
    filter: (linear | nearest)
    wrap_mode: (clamp | repeat)
  raw_spectrum:
    mipmap: bool
    filter: (linear | nearest)
    wrap_mode: (clamp | repeat)
  samples:
    mipmap: bool
    filter: (linear | nearest)
    wrap_mode: (clamp | repeat)
```

すべてのオーディオテキスチャはfloatです。

## 共通uniform

```glsl
// ターゲットの解像度
uniform vec4 resolution; // vec4(x, y, x/y, y/x)

// ステージのインデックス
// 同じシェーダーを複数回実行する場合に便利かもしれません
uniform int pass_index;

// プログラム起動時からの秒数
uniform float time;

// 前フレームからの経過時間
uniform float time_delta;

// beat == time * BPM / 60
// BPMはコントロールパネルから設定できます。
uniform float beat;

// コントロールパネルにあるスライダーの値に対応します
uniform float sliders[32];

// コントロールパネルにあるボタンに対応します
// buttons[i] = vec4(intensity, since_last_on, since_last_off, count);
// intensity: NoteOnのvelocityとPolyphonicKeyPressureの値が書き込まれます
// since_last_on: 直近の NoteOn からの経過秒数
// since_last_off: 直近の NoteOff からの経過秒数
// count: NoteOnが何回発行されたかを数え上げる整数値
uniform vec4 buttons[32];

// 32x32x32の乱数テキスチャ。
// パイプラインが読み込まれるたびに再計算されるので
// 　コンパイルなどを走らせるとテキスチャの中身が変わります。
uniform sampler3D noise;

// プログラム開始からのフレーム数
uniform int frame_count;

// オーディオ入力デバイスからの生サンプル.
// r には左チャンネル (モノラルの場合は唯一)　の情報が入ります
// g には右チャンネルの情報が入ります
uniform sampler1D samples;

// 生FFT情報
// r/g は上記と同じく
uniform sampler1D spectrum_raw;

// "いい感じ"なFFT、EQをかけたり音階にゆるく対応しています。
// r/g は上記の同じく
uniform sampler1D spectrum;
uniform sampler1D spectrum_smooth;
uniform sampler1D spectrum_integrated;
uniform sampler1D spectrum_smooth_integrated;

// Bass/Mid/High
uniform vec3 bass;
uniform vec3 bass_smooth;
uniform vec3 bass_integrated;
uniform vec3 bass_smooth_integrated;

uniform vec3 mid;
uniform vec3 mid_smooth;
uniform vec3 mid_integrated;
uniform vec3 mid_smooth_integrated;

uniform vec3 high;
uniform vec3 high_smooth;
uniform vec3 high_integrated;
uniform vec3 high_smooth_integrated;

// 現在の音量, 全サンプルのRMSで計算されてます
// r には左右の平均値、モノラルの場合は音量が入ります
// g には左チャンネルの音量が入ります
// b には右チャンネルの音量が入ります
uniform vec3 volume;
uniform vec3 volume_integrated;
```

さらに、パイプラインファイルの`uniforms`フィールドを使用して、カスタムuniformsを任意のシェーダーステージに追加できます。

```yaml
stages:
  - fs: "scene.frag"
    uniforms:
      - strength: 2.3
      - iter: 20
      - color: [1, 0.4, 0.7]
      - rotation^T: [[0.9, 0.2], [-0.2, 0.9]]
```

これらは、次のようにシェーダーでアクセスできます。
uniformのすべての数字は、型を簡単に推測できるように、floatとして解釈されることに注意してください。

```glsl
uniform float strength;
uniform float iter;
uniform vec3 color;
uniform mat2 rotation;
```

また、ここでは`rotation`という行列がtransposeされていることに注意してください。
普通では、行列はrow-majorに解釈されます。 それらがtransposeされる場合、それらはcolumn-majorです。

## Hotkeys

| key combination　| feature |
| --- | --- |
| ctrl + enter | パイプラインのビルド |
| alt + enter | borderless windowed モード切り替え |
| shift + ctrl + s | スクリーンショットを撮って、cwd以下に配置されます |
