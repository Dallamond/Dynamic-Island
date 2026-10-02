//! Volumen y salida de audio (WASAPI / Core Audio).
//!
//! Sin hilo propio: solo se consulta cuando el panel de música está abierto (lo pide el frontend).
//! Cambiar la salida por defecto usa `IPolicyConfig`, una interfaz COM no documentada pero
//! estable desde Windows 7, que es la que usan todas las utilidades de cambio de audio.

#![allow(non_snake_case)]

use serde::Serialize;
use windows::core::{GUID, HRESULT, PCWSTR, PWSTR};
use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{
    eCommunications, eConsole, eMultimedia, eRender, ERole, IMMDevice, IMMDeviceEnumerator, MMDeviceEnumerator,
    DEVICE_STATE_ACTIVE,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, COINIT_MULTITHREADED, STGM_READ,
};

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AudioDevice {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AudioState {
    pub volume: f32,
    pub muted: bool,
    pub devices: Vec<AudioDevice>,
}

fn com_init() {
    unsafe {
        // S_FALSE / RPC_E_CHANGED_MODE si ya estaba inicializado: ambos valen.
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
}

fn enumerator() -> windows::core::Result<IMMDeviceEnumerator> {
    com_init();
    unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) }
}

fn endpoint_volume() -> windows::core::Result<IAudioEndpointVolume> {
    unsafe {
        let dev = enumerator()?.GetDefaultAudioEndpoint(eRender, eConsole)?;
        dev.Activate(CLSCTX_ALL, None)
    }
}

fn device_id(dev: &IMMDevice) -> windows::core::Result<String> {
    unsafe {
        let p: PWSTR = dev.GetId()?;
        let s = p.to_string().unwrap_or_default();
        CoTaskMemFree(Some(p.0 as _));
        Ok(s)
    }
}

fn device_name(dev: &IMMDevice) -> String {
    unsafe {
        dev.OpenPropertyStore(STGM_READ)
            .and_then(|store| store.GetValue(&PKEY_Device_FriendlyName))
            .map(|v| v.to_string())
            .unwrap_or_else(|_| "Dispositivo".into())
    }
}

pub fn state() -> windows::core::Result<AudioState> {
    unsafe {
        let en = enumerator()?;
        let default_id = en.GetDefaultAudioEndpoint(eRender, eConsole).and_then(|d| device_id(&d)).unwrap_or_default();
        let col = en.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)?;
        let mut devices = Vec::new();
        for i in 0..col.GetCount()? {
            let d = col.Item(i)?;
            let id = device_id(&d)?;
            devices.push(AudioDevice { is_default: id == default_id, name: device_name(&d), id });
        }
        let vol = endpoint_volume()?;
        Ok(AudioState { volume: vol.GetMasterVolumeLevelScalar()?, muted: vol.GetMute()?.as_bool(), devices })
    }
}

pub fn set_volume(v: f32) -> windows::core::Result<()> {
    unsafe { endpoint_volume()?.SetMasterVolumeLevelScalar(v.clamp(0.0, 1.0), std::ptr::null()) }
}

pub fn set_mute(m: bool) -> windows::core::Result<()> {
    unsafe { endpoint_volume()?.SetMute(m, std::ptr::null()) }
}

// ---------------------------------------------------------------- IPolicyConfig (no documentada)

/// Vtable de IPolicyConfig: solo se llama a SetDefaultEndpoint, el resto son huecos
/// para respetar el orden de la vtable.
#[windows::core::interface("f8679f50-850a-41cf-9c72-430f290290c8")]
unsafe trait IPolicyConfig: windows::core::IUnknown {
    fn GetMixFormat(&self) -> HRESULT;
    fn GetDeviceFormat(&self) -> HRESULT;
    fn ResetDeviceFormat(&self) -> HRESULT;
    fn SetDeviceFormat(&self) -> HRESULT;
    fn GetProcessingPeriod(&self) -> HRESULT;
    fn SetProcessingPeriod(&self) -> HRESULT;
    fn GetShareMode(&self) -> HRESULT;
    fn SetShareMode(&self) -> HRESULT;
    fn GetPropertyValue(&self) -> HRESULT;
    fn SetPropertyValue(&self) -> HRESULT;
    fn SetDefaultEndpoint(&self, device_id: PCWSTR, role: ERole) -> HRESULT;
    fn SetEndpointVisibility(&self) -> HRESULT;
}

const CLSID_POLICY_CONFIG_CLIENT: GUID = GUID::from_u128(0x870af99c_171d_4f9e_af0d_e63df40c2bc9);

pub fn set_default_device(id: &str) -> windows::core::Result<()> {
    com_init();
    let wide: Vec<u16> = id.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let pc: IPolicyConfig = CoCreateInstance(&CLSID_POLICY_CONFIG_CLIENT, None, CLSCTX_ALL)?;
        for role in [eConsole, eMultimedia, eCommunications] {
            pc.SetDefaultEndpoint(PCWSTR(wide.as_ptr()), role).ok()?;
        }
    }
    Ok(())
}
