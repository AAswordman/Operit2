// Copyright 2025, Colin McKee
// SPDX-License-Identifier: Apache-2.0
// Ported from FletchMcKee/liquid 1.1.1, internal/LiquidShader.kt.
// Flutter adaptation: shared background sampler and affine source mapping.
#include <flutter/runtime_effect.glsl>
precision highp float;

uniform vec4 uBounds;      // size.xy, paint offset.xy (logical canvas coordinates)
uniform vec4 uCornerRadii; // bottom-right, top-right, bottom-left, top-left
uniform vec4 uLens;        // refraction, curve, edge, reserved
uniform vec4 uTint;
uniform vec4 uColor;       // saturation, dispersion, contrast, reserved
uniform vec4 uBackground;  // local origin in source.xy, source size.xy
uniform vec4 uBasis;       // local X axis.xy, local Y axis.xy in source space
uniform sampler2D uContent;
out vec4 fragColor;

/// Returns the signed distance to the actual corner in each quadrant.
float computeSdf(vec2 p, vec2 b, vec4 r) {
  r.xy = p.x > 0.0 ? r.xy : r.zw;
  r.x = p.y > 0.0 ? r.x : r.y;
  vec2 q = abs(p) - b + r.x;
  return min(max(q.x, q.y), 0.0) + length(max(q, 0.0)) - r.x;
}

/// Computes an analytical normal, including the straight sides of the lens.
vec2 computeSdfNormal(vec2 pos, vec2 halfSize, vec4 r) {
  r.xy = pos.x > 0.0 ? r.xy : r.zw;
  r.x = pos.y > 0.0 ? r.x : r.y;
  vec2 w = abs(pos) - (halfSize - r.x);
  vec2 s = vec2(pos.x < 0.0 ? -1.0 : 1.0, pos.y < 0.0 ? -1.0 : 1.0);
  vec2 q = max(w, 0.0);
  return s * (max(w.x, w.y) > 0.0
    ? q / length(q)
    : (w.x > w.y ? vec2(1.0, 0.0) : vec2(0.0, 1.0)));
}

/// Maps local lens coordinates through the source's actual affine transform.
vec2 sourceUv(vec2 p) {
  vec2 uv = (uBackground.xy + uBasis.xy * p.x + uBasis.zw * p.y) / uBackground.zw;
  // Scene.toImageSync produces a render-target texture. Impeller's GLES target
  // stores those rows bottom-up; keep geometry and refraction in logical space
  // and convert only the final source UV using Flutter's shader backend define.
#ifdef IMPELLER_TARGET_OPENGLES
  uv.y = 1.0 - uv.y;
#endif
  return uv;
}

/// Samples the clear source at the refracted coordinate without spatial blur.
vec4 sampleContent(vec2 p) {
  return texture(uContent, sourceUv(p));
}
/// Applies upstream's luminance-preserving saturation and contrast operations.
vec3 applyColorAdjustments(vec3 color) {
  float lum = dot(color, vec3(0.2126, 0.7152, 0.0722));
  color = clamp(mix(vec3(lum), color, uColor.x), 0.0, 1.0);
  return clamp((color - 0.5) * uColor.z + 0.5, 0.0, 1.0);
}

/// Refracts the source through a curved lens and adds directional edge light.
void main() {
  vec2 size = uBounds.xy;
  vec2 fragCoord = FlutterFragCoord().xy - uBounds.zw;
  float minDimension = min(size.x, size.y);
  vec2 center = size * 0.5;
  vec2 shapeCoord = (fragCoord - center) / minDimension;
  vec2 halfSize = size / minDimension * 0.5;
  float shapeSdf = computeSdf(shapeCoord, halfSize, uCornerRadii);
  float aaWidth = 1.5 / minDimension;
  if (shapeSdf > aaWidth) {
    fragColor = vec4(0.0);
    return;
  }

  // The box normal changes axes on its interior diagonals. Keep those switches
  // outside the active refraction band, where displacement is already zero.
  // Only the normal field changes; shapeSdf retains the actual corner radii.
  vec4 lensRadii = min(max(uCornerRadii * 1.5, vec4(uLens.x)), vec4(0.5));
  vec2 normal = computeSdfNormal(shapeCoord, halfSize, lensRadii);
  vec2 baseCoord = fragCoord;
  if (uLens.x > 0.0 && uLens.y > 0.0) {
    float lensDepth = 1.0 - clamp(-shapeSdf / uLens.x, 0.0, 1.0);
    float distortion = 1.0 - sqrt(max(0.0, 1.0 - lensDepth * lensDepth));
    baseCoord += distortion * -uLens.y * minDimension * normal;
  }

  vec4 color = sampleContent(baseCoord);
  if (uColor.y > 0.0) {
    vec2 fromCenter = (fragCoord - center) / size;
    vec2 aberration = uColor.y * fromCenter * fromCenter * fromCenter * minDimension;
    color.r = sampleContent(baseCoord - aberration).r;
    color.b = sampleContent(baseCoord + aberration).b;
  }

  // Background coverage is independent of the material tint's opacity.
  vec3 rgb = color.rgb / max(color.a, 0.00001);
  rgb = applyColorAdjustments(rgb);
  rgb = mix(rgb, uTint.rgb, uTint.a);
  if (uLens.z > 0.0) {
    float edgeSmooth = smoothstep(-uLens.z, 0.0, shapeSdf);
    rgb += edgeSmooth * abs(dot(normal, vec2(-0.15, -0.15)));
  }
  float alpha = color.a * (1.0 - smoothstep(-aaWidth * 0.5, aaWidth * 0.5, shapeSdf));
  fragColor = vec4(clamp(rgb, 0.0, 1.0) * alpha, alpha);
}
