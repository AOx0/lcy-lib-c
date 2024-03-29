use nalgebra::{dmatrix, DMatrix};
use rand::rngs::ThreadRng;
use rand::Rng;
use std::collections::HashMap;

#[repr(C)]
pub struct DynArray {
    array: *mut u8,
    length: libc::size_t,
}

/// # Safety
#[no_mangle]
pub unsafe extern "C" fn decipher_bytes(array: *mut u8, size: u32) -> DynArray {
    let contenidos = {
        assert!(!array.is_null());
        std::slice::from_raw_parts(array, size as usize)
    };

    let mut map: HashMap<u8, u8> = HashMap::new();

    if contenidos[0..0x4] != [66u8, 60u8, 10u8, 255u8] {
        eprintln!("Error: No es un archivo válido cifrado");
    }

    let mut transformation_inverse = dmatrix![].resize(32, 32, 0.0);
    let mut cyphered_bytes = dmatrix![].resize(8, 32, 0.0);

    let contenidos = &contenidos[0x4..];

    let mut expanded = vec![];
    for i in 0..256 {
        let str_form = format!("{:#010b}", contenidos[i])
          .replace("0b", "")
          .chars()
          .collect::<Vec<char>>()
          ;

        let mut tmp = [0u8; 4];
        let mut numbers: Vec<u8> = str_form.
          into_iter()
          .map(|v| {
              tmp = [0,0,0,0];
              v.encode_utf8(&mut tmp).parse().unwrap()
          })
          .collect();

        expanded.append(&mut numbers);
    }

    expanded.extend_from_slice(&contenidos[256..]);

    let contenidos = expanded;

    let mut k = 0;
    for i in 0..32 {
        for j in 0..32 {
            let neg = contenidos[k] == 0x1;
            let val = (contenidos[k + 1] as i32) * if neg { -1 } else { 1 };

            transformation_inverse[(i, j)] = val as f32;
            k += 2;
        }
    }

    let contenidos = &contenidos[k..];

    let mut k = 0;
    for i in 0..8 {
        for j in 0..32 {
            cyphered_bytes[(i, j)] = contenidos[k] as f32;
            k += 1;
        }
    }

    let mut orig = &cyphered_bytes * &transformation_inverse;

    for i in 0..8usize {
        for j in 0..32usize {
            orig[(i, j)] = orig[(i, j)].rem_euclid(256.0);
            map.insert(cyphered_bytes[(i, j)] as u8, orig[(i, j)] as u8);
        }
    }

    let mut contenidos = contenidos[k..].to_vec();

    for i_byte in 0..contenidos.len() {
        contenidos[i_byte] = *map.get(&contenidos[i_byte]).unwrap();
    }

    let result = DynArray {
        array: contenidos.as_mut_ptr(),
        length: contenidos.len() as _,
    };

    std::mem::forget(contenidos);

    result
}

/// # Safety
#[no_mangle]
pub unsafe extern "C" fn cypher_bytes(array: *mut u8, size: u32) -> DynArray {
    let contenidos_r = {
        assert!(!array.is_null());
        std::slice::from_raw_parts(array, size as usize)
    };

    let mut contenidos = contenidos_r.to_vec();

    let mut rng = rand::thread_rng();

    let mut map: HashMap<u8, u8> = HashMap::new();
    let bytes = craft_bytes_matrix();
    let mut resulting_bytes: Vec<u8> = vec![];

    let transformation = met1_armar_matriz(&mut rng);
    let mut cyphered_bytes = &bytes * &transformation;
    let transformation_inverse = transformation.try_inverse().unwrap();

    for i in 0..8usize {
        for j in 0..32usize {
            cyphered_bytes[(i, j)] = cyphered_bytes[(i, j)].rem_euclid(256.0);
            map.insert(bytes[(i, j)] as u8, cyphered_bytes[(i, j)] as u8);
        }
    }

    for i_byte in 0..contenidos.len() {
        contenidos[i_byte] = *map.get(&contenidos[i_byte]).unwrap();
    }

    let mut inv_key_to_write = vec![];
    let mut bytes_final_to_write = vec![];
    for i in 0..32 {
        for j in 0..32 {
            let indicador = if transformation_inverse[(i, j)] < 0.0 {
                1u8
            } else {
                0u8
            };
            let val = if transformation_inverse[(i, j)] < 0.0 {
                -1.0 * transformation_inverse[(i, j)]
            } else {
                transformation_inverse[(i, j)]
            };

            inv_key_to_write.push(indicador);
            inv_key_to_write.push(val as u8);
        }
    }

    let mut inv_key_to_write_2 = vec![];
    let mut temp = String::new();
    for (mut num, &val) in inv_key_to_write.iter().enumerate() {
        num += 1;
        temp += if val == 1 { "1" } else { "0" };

        if num % 8 == 0 {
            inv_key_to_write_2.push(u8::from_str_radix(&temp, 2).unwrap());
            temp.clear();
        }
    }

    for i in 0..8 {
        for j in 0..32 {
            bytes_final_to_write.push(cyphered_bytes[(i, j)] as u8);
        }
    }

    resulting_bytes.append(&mut vec![66u8, 60u8, 10u8, 255u8]);
    resulting_bytes.append(&mut inv_key_to_write_2);
    resulting_bytes.append(&mut bytes_final_to_write);
    resulting_bytes.append(&mut contenidos);

    drop(map);
    drop(contenidos);
    drop(transformation_inverse);

    let result = DynArray {
        array: resulting_bytes.as_mut_ptr(),
        length: resulting_bytes.len() as _,
    };

    std::mem::forget(resulting_bytes);

    result
}

pub fn met1_armar_matriz(rng: &mut ThreadRng) -> DMatrix<f32> {
    let mut resultado: DMatrix<f32> = dmatrix![].resize(32, 32, 0.0);

    let mut switch: [bool; 32] = [false; 32];
    let mut neg: [bool; 32] = [false; 32];

    for col in 1..32 / 2 {
        if rng.gen::<u8>() % 2 == 0 {
            switch[col] = true;
            switch[31 - (col - 1)] = true;
        }

        if rng.gen::<u8>() % 5 == 0 {
            neg[col] = true;
            neg[31 - (col - 1)] = true;
        }
    }

    resultado[(0, 0)] = 1.0;

    for col in 1..32 {
        if switch[col] {
            resultado[(col, 31 - (col - 1))] = if neg[col] { 1.0 } else { -1.0 };
        } else {
            resultado[(col, col)] = if neg[col] { 1.0 } else { -1.0 };
        }
    }

    resultado
}

pub fn craft_bytes_matrix() -> DMatrix<f32> {
    let mut bytes: DMatrix<f32> = dmatrix![].resize(8, 32, 0.0);

    let mut k = 0;
    for i in 0..8 {
        for j in 0..32 {
            bytes[(i, j)] = k as f32;
            k += 1;
        }
    }
    bytes
}
