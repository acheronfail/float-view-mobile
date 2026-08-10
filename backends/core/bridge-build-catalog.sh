# Shared allowlist for the retained Rust host bridges.
# This file is sourced by the Apple and Android builders and is intentionally
# non-executable. Callers select only a stable bridge identifier; package,
# artifact, diagnostic, and JNI symbol metadata never comes from free-form
# command input.

readonly float_view_rust_toolchain='1.93.0'
readonly float_view_android_ndk_version='28.2.13676358'
readonly float_view_android_api='29'

float_view_select_bridge() {
  local requested_bridge="${1:-}"

  bridge_identifier=''
  bridge_diagnostic=''
  bridge_apple_package=''
  bridge_apple_archive=''
  bridge_android_package=''
  bridge_android_library_stem=''
  bridge_android_jni_prefix=''
  bridge_android_symbols=()

  case "$requested_bridge" in
    frame)
      bridge_identifier='frame'
      bridge_diagnostic='VESC frame codec'
      bridge_apple_package='float-view-vesc-frame-apple-ffi'
      bridge_apple_archive='libfloat_view_vesc_frame_apple_ffi.a'
      bridge_android_package='float-view-vesc-frame-android-jni'
      bridge_android_library_stem='float_view_vesc_frame_android_jni'
      bridge_android_jni_prefix='Java_fail_acheron_floatview_protocol_RustVescFrameBridge'
      bridge_android_symbols=(
        Java_fail_acheron_floatview_protocol_RustVescFrameBridge_abiVersion
        Java_fail_acheron_floatview_protocol_RustVescFrameBridge_decodeExact
        Java_fail_acheron_floatview_protocol_RustVescFrameBridge_encode
        Java_fail_acheron_floatview_protocol_RustVescFrameBridge_scanNext
      )
      ;;
    firmware-identity)
      bridge_identifier='firmware-identity'
      bridge_diagnostic='VESC firmware identity'
      bridge_apple_package='float-view-vesc-firmware-identity-apple-ffi'
      bridge_apple_archive='libfloat_view_vesc_firmware_identity_apple_ffi.a'
      bridge_android_package='float-view-vesc-firmware-identity-android-jni'
      bridge_android_library_stem='float_view_vesc_firmware_identity_android_jni'
      bridge_android_jni_prefix='Java_fail_acheron_floatview_protocol_RustVescFirmwareIdentityBridge'
      bridge_android_symbols=(
        Java_fail_acheron_floatview_protocol_RustVescFirmwareIdentityBridge_abiVersion
        Java_fail_acheron_floatview_protocol_RustVescFirmwareIdentityBridge_decode
      )
      ;;
    custom-configuration)
      bridge_identifier='custom-configuration'
      bridge_diagnostic='VESC custom configuration'
      bridge_apple_package='float-view-vesc-custom-configuration-apple-ffi'
      bridge_apple_archive='libfloat_view_vesc_custom_configuration_apple_ffi.a'
      bridge_android_package='float-view-vesc-custom-configuration-android-jni'
      bridge_android_library_stem='float_view_vesc_custom_configuration_android_jni'
      bridge_android_jni_prefix='Java_fail_acheron_floatview_protocol_RustVescCustomConfigurationBridge'
      bridge_android_symbols=(
        Java_fail_acheron_floatview_protocol_RustVescCustomConfigurationBridge_abiVersion
        Java_fail_acheron_floatview_protocol_RustVescCustomConfigurationBridge_decode
        Java_fail_acheron_floatview_protocol_RustVescCustomConfigurationBridge_encode
      )
      ;;
    bms-values)
      bridge_identifier='bms-values'
      bridge_diagnostic='VESC BMS values'
      bridge_apple_package='float-view-vesc-bms-values-apple-ffi'
      bridge_apple_archive='libfloat_view_vesc_bms_values_apple_ffi.a'
      bridge_android_package='float-view-vesc-bms-values-android-jni'
      bridge_android_library_stem='float_view_vesc_bms_values_android_jni'
      bridge_android_jni_prefix='Java_fail_acheron_floatview_protocol_RustVescBmsValuesBridge'
      bridge_android_symbols=(
        Java_fail_acheron_floatview_protocol_RustVescBmsValuesBridge_abiVersion
        Java_fail_acheron_floatview_protocol_RustVescBmsValuesBridge_decode
      )
      ;;
    controller-data)
      bridge_identifier='controller-data'
      bridge_diagnostic='VESC controller data'
      bridge_apple_package='float-view-vesc-controller-data-apple-ffi'
      bridge_apple_archive='libfloat_view_vesc_controller_data_apple_ffi.a'
      bridge_android_package='float-view-vesc-controller-data-android-jni'
      bridge_android_library_stem='float_view_vesc_controller_data_android_jni'
      bridge_android_jni_prefix='Java_fail_acheron_floatview_protocol_RustVescControllerDataBridge'
      bridge_android_symbols=(
        Java_fail_acheron_floatview_protocol_RustVescControllerDataBridge_abiVersion
        Java_fail_acheron_floatview_protocol_RustVescControllerDataBridge_decode
      )
      ;;
    refloat-control)
      bridge_identifier='refloat-control'
      bridge_diagnostic='Refloat control codec'
      bridge_apple_package='float-view-refloat-control-apple-ffi'
      bridge_apple_archive='libfloat_view_refloat_control_apple_ffi.a'
      bridge_android_package='float-view-refloat-control-android-jni'
      bridge_android_library_stem='float_view_refloat_control_android_jni'
      bridge_android_jni_prefix='Java_fail_acheron_floatview_protocol_RustRefloatControlBridge'
      bridge_android_symbols=(
        Java_fail_acheron_floatview_protocol_RustRefloatControlBridge_abiVersion
        Java_fail_acheron_floatview_protocol_RustRefloatControlBridge_decodeLighting
        Java_fail_acheron_floatview_protocol_RustRefloatControlBridge_encode
      )
      ;;
    *)
      echo "Unknown Rust host bridge: ${requested_bridge:-<empty>}" >&2
      return 2
      ;;
  esac
}
