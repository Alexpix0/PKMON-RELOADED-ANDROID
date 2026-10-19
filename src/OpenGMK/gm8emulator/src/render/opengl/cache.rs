//! Persistent Binary Shader Cache for OpenGL ES and Desktop OpenGL.
//!
//! Serializes compiled and linked GL program binaries to the application's `code_cache/`
//! using `glProgramBinary` and `glGetProgramBinary` (core in OpenGL ES 3.0+ / OpenGL 4.1+).
//!
//! Uses a deterministic hash of the GPU vendor, renderer, driver version, app version,
//! and GLSL source to ensure zero compilation stutter while automatically invalidating
//! stale caches upon system or app updates.

use std::{
    fs::{self, File},
    io::{Read, Write},
    path::PathBuf,
    ptr,
};
use super::gl;
use super::gl::types::*;

pub struct ShaderCache;

impl ShaderCache {
    /// Computes a deterministic 64-bit FNV-1a hash over all input strings.
    pub fn compute_hash(inputs: &[&str]) -> u64 {
        const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
        const FNV_PRIME: u64 = 0x100000001b3;

        let mut hash = FNV_OFFSET_BASIS;
        for input in inputs {
            for byte in input.as_bytes() {
                hash ^= *byte as u64;
                hash = hash.wrapping_mul(FNV_PRIME);
            }
        }
        hash
    }

    /// Gets the cache directory for compiled shader binaries.
    pub fn cache_dir() -> PathBuf {
        #[cfg(target_os = "android")]
        {
            let path = PathBuf::from("/data/data/com.klisee.reloaded/code_cache");
            if path.exists() {
                return path;
            }
            let fallback = PathBuf::from("/data/data/com.klisee/code_cache");
            if fallback.exists() {
                return fallback;
            }
        }
        let local = PathBuf::from("code_cache");
        let _ = fs::create_dir_all(&local);
        local
    }

    /// Attempts to load a cached binary program, or compiles, links, and caches it.
    pub unsafe fn load_or_compile(
        gl: &gl::Gl,
        name: &str,
        vertex_source: &str,
        fragment_source: &str,
    ) -> Result<GLuint, String> {
        // Query GPU identification strings
        let renderer = {
            let p = gl.GetString(gl::RENDERER);
            if p.is_null() { "unknown_renderer" } else { std::ffi::CStr::from_ptr(p.cast()).to_str().unwrap_or("unknown") }
        };
        let version = {
            let p = gl.GetString(gl::VERSION);
            if p.is_null() { "unknown_version" } else { std::ffi::CStr::from_ptr(p.cast()).to_str().unwrap_or("unknown") }
        };

        let app_version = "0.9.0";
        let hash = Self::compute_hash(&[name, renderer, version, app_version, vertex_source, fragment_source]);
        let cache_file = Self::cache_dir().join(format!("prog_{}_{:016x}.bin", name, hash));

        // 1. Try loading from cache if binary functions are supported
        if gl.ProgramBinary.is_loaded() && cache_file.exists() {
            if let Ok(mut file) = File::open(&cache_file) {
                let mut header = [0u8; 16]; // [u64 hash, u32 format, u32 length]
                if file.read_exact(&mut header).is_ok() {
                    let file_hash = u64::from_le_bytes(header[0..8].try_into().unwrap());
                    let binary_format = u32::from_le_bytes(header[8..12].try_into().unwrap());
                    let length = u32::from_le_bytes(header[12..16].try_into().unwrap()) as usize;

                    if file_hash == hash && length > 0 && length < 16 * 1024 * 1024 {
                        let mut binary_data = vec![0u8; length];
                        if file.read_exact(&mut binary_data).is_ok() {
                            let program = gl.CreateProgram();
                            gl.ProgramBinary(
                                program,
                                binary_format,
                                binary_data.as_ptr().cast(),
                                length as GLsizei,
                            );

                            let mut success: GLint = 0;
                            gl.GetProgramiv(program, gl::LINK_STATUS, &mut success);
                            if success == gl::TRUE as GLint {
                                eprintln!("[shader_cache] Programa '{}' cargado instantáneamente desde binario ({:016x})", name, hash);
                                return Ok(program);
                            }
                            // If invalid or stale, delete and recompile
                            gl.DeleteProgram(program);
                        }
                    }
                }
            }
        }

        // 2. Cache miss or unsupported: Compile and link from GLSL sources
        let vertex_shader = gl.CreateShader(gl::VERTEX_SHADER);
        gl.ShaderSource(vertex_shader, 1, &(vertex_source.as_ptr().cast()), ptr::null());
        gl.CompileShader(vertex_shader);

        let mut success: GLint = 0;
        gl.GetShaderiv(vertex_shader, gl::COMPILE_STATUS, &mut success);
        if success != gl::TRUE as GLint {
            let mut info_len: GLint = 0;
            gl.GetShaderiv(vertex_shader, gl::INFO_LOG_LENGTH, &mut info_len);
            let mut info = vec![0u8; info_len as usize];
            gl.GetShaderInfoLog(vertex_shader, info_len as GLsizei, ptr::null_mut(), info.as_mut_ptr().cast());
            return Err(format!("Shader de vertices ({}) fallo: {}", name, String::from_utf8_lossy(&info)));
        }

        let fragment_shader = gl.CreateShader(gl::FRAGMENT_SHADER);
        gl.ShaderSource(fragment_shader, 1, &(fragment_source.as_ptr().cast()), ptr::null());
        gl.CompileShader(fragment_shader);

        gl.GetShaderiv(fragment_shader, gl::COMPILE_STATUS, &mut success);
        if success != gl::TRUE as GLint {
            let mut info_len: GLint = 0;
            gl.GetShaderiv(fragment_shader, gl::INFO_LOG_LENGTH, &mut info_len);
            let mut info = vec![0u8; info_len as usize];
            gl.GetShaderInfoLog(fragment_shader, info_len as GLsizei, ptr::null_mut(), info.as_mut_ptr().cast());
            return Err(format!("Shader de fragmentos ({}) fallo: {}", name, String::from_utf8_lossy(&info)));
        }

        let program = gl.CreateProgram();
        // Hint the driver that this program should be retrievable as a binary
        const PROGRAM_BINARY_RETRIEVABLE_HINT: GLenum = 0x8257;
        if gl.ProgramParameteri.is_loaded() {
            gl.ProgramParameteri(program, PROGRAM_BINARY_RETRIEVABLE_HINT, gl::TRUE as GLint);
        }

        gl.AttachShader(program, vertex_shader);
        gl.AttachShader(program, fragment_shader);
        gl.LinkProgram(program);

        gl.GetProgramiv(program, gl::LINK_STATUS, &mut success);
        if success != gl::TRUE as GLint {
            let mut info_len: GLint = 0;
            gl.GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut info_len);
            let mut info = vec![0u8; info_len as usize];
            gl.GetProgramInfoLog(program, info_len as GLsizei, ptr::null_mut(), info.as_mut_ptr().cast());
            return Err(format!("Enlazado de programa ({}) fallo: {}", name, String::from_utf8_lossy(&info)));
        }

        gl.DeleteShader(vertex_shader);
        gl.DeleteShader(fragment_shader);

        // 3. Save program binary to cache if supported
        const PROGRAM_BINARY_LENGTH: GLenum = 0x8741;
        if gl.GetProgramBinary.is_loaded() {
            let mut binary_length: GLint = 0;
            gl.GetProgramiv(program, PROGRAM_BINARY_LENGTH, &mut binary_length);
            if binary_length > 0 {
                let mut binary = vec![0u8; binary_length as usize];
                let mut written: GLsizei = 0;
                let mut binary_format: GLenum = 0;
                gl.GetProgramBinary(
                    program,
                    binary_length,
                    &mut written,
                    &mut binary_format,
                    binary.as_mut_ptr().cast(),
                );

                if written > 0 {
                    let mut file_data = Vec::with_capacity(16 + written as usize);
                    file_data.extend_from_slice(&hash.to_le_bytes());
                    file_data.extend_from_slice(&(binary_format as u32).to_le_bytes());
                    file_data.extend_from_slice(&(written as u32).to_le_bytes());
                    file_data.extend_from_slice(&binary[..written as usize]);

                    if let Ok(mut f) = File::create(&cache_file) {
                        let _ = f.write_all(&file_data);
                        eprintln!("[shader_cache] Guardado binario persistente para '{}' ({} bytes, hash: {:016x})", name, written, hash);
                    }
                }
            }
        }

        Ok(program)
    }
}
