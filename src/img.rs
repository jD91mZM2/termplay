use clap::ArgMatches;
use colors::*;
use image;
use image::{DynamicImage, FilterType, GenericImage, Pixel};
use sixel_sys;
use std::collections::HashMap;
use std::io;
use std::io::Write;
use std::os::raw::{c_char, c_int, c_uchar, c_void};
use std::ptr;
use std::slice;
use std::str::FromStr;
use terminal_size::terminal_size;

#[macro_export]
macro_rules! fit_and_convert {
	($image:expr, $converter:expr, $width:expr, $height:expr, $ratio:expr, $keep_size:expr) => {
		{
			let mut image = $image;
			if !$keep_size {
				image = ::img::fit(&image, $converter, $width, $height, $ratio);
			}
			::img::convert(&image, $converter, $ratio)
		}
	}
}

pub fn main(options: &ArgMatches) -> i32 {
	let image_path = options.value_of("IMAGE").unwrap();

	make_parse_macro!(options);
	let width = parse!("width", u16);
	let height = parse!("height", u16);
	let ratio = parse!("ratio", u8).unwrap();
	let keep_size = options.is_present("keep-size");
	let converter = options.value_of("converter").unwrap().parse().unwrap();

	let image = match image::open(image_path) {
		Ok(image) => image,
		Err(err) => {
			stderr!("Could not open image.");
			stderr!("{}", err);
			return 1;
		},
	};

	println!(
		"{}",
		fit_and_convert!(image, converter, width, height, ratio, keep_size)
	);

	0
}

pub fn fit(image: &DynamicImage, converter: Converter, width: Option<u16>, height: Option<u16>, ratio: u8) -> DynamicImage {
	let mut term_width = None;
	let mut term_height = None;

	if width.is_none() || height.is_none() {
		if let Some((width, height)) = terminal_size() {
			term_width = Some(width.0);
			term_height = Some(height.0);
		} else {
			term_width = Some(40);
			term_height = Some(20);
		}
	}
	let mut width = match width {
		Some(width) => width,
		None => term_width.unwrap(),
		// It's safe to assume unwrap(), since we do fill them in if anything is None
	} as u32;
	let mut height = match height {
		Some(height) => height,
		None => term_height.unwrap(),
	} as u32;

	if converter == Converter::Sixel {
		width *= 10;
		height *= 10;
	} else {
		height = (height as f32 * (ratio as f32 / 100.0 + 1.0)) as u32;
	}
	image.resize(width, height, FilterType::Nearest)
}

#[derive(PartialEq, Clone, Copy)]
pub enum Converter {
	TrueColor,
	Color256,
	Sixel
}
impl FromStr for Converter {
	type Err = ();

	fn from_str(converter: &str) -> Result<Self, Self::Err> {
		match converter {
			"truecolor" => Ok(Converter::TrueColor),
			"256-color" => Ok(Converter::Color256),
			"sixel" => Ok(Converter::Sixel),
			_ => Err(()),
		}
	}
}
pub fn convert(image: &DynamicImage, converter: Converter, ratio: u8) -> String {
	match converter {
		Converter::TrueColor => convert_true(image, ratio),
		Converter::Color256 => convert_256(image, ratio),
		Converter::Sixel => convert_sixel(image),
	}
}
pub fn convert_true(image: &DynamicImage, ratio: u8) -> String {
	// This allocation isn't enough, but it's at least some help along the way.
	let (width, height) = (image.width(), image.height());
	let mut result = String::with_capacity(
		(width as usize + 1) * height as usize * 16 + COLOR_RESET.len()
	);
	// width + 1 because newline
	// 16 = e[38;2;0;0;0m█
	// (block is 3 bytes)

	let mut ratio_issues = 0;

	for y in 0..image.height() {
		ratio_issues += ratio;
		if ratio_issues >= 100 {
			ratio_issues -= 100;
			continue;
		}

		for x in 0..image.width() {
			let pixel = image.get_pixel(x, y);
			let channels = pixel.channels();

			result.push_str("\x1b[38;2;");
			result.push_str(channels[0].to_string().as_str());
			result.push(';');
			result.push_str(channels[1].to_string().as_str());
			result.push(';');
			result.push_str(channels[2].to_string().as_str());
			result.push('m');
			result.push('█');
		}
		result.push('\n');
	}

	result.push_str(COLOR_RESET);
	result
}
pub fn convert_256(image: &DynamicImage, ratio: u8) -> String {
	// This allocation isn't enough, but it's at least some help along the way.
	let (width, height) = (image.width(), image.height());
	let mut result = String::with_capacity(
		(width as usize + 1) * height as usize * 16 + COLOR_RESET.len()
	);
	// width + 1 because newline
	// 12 = e[38;5;0m█
	// (block is 3 bytes)

	let mut ratio_issues = 0;

	for y in 0..image.height() {
		ratio_issues += ratio;
		if ratio_issues >= 100 {
			ratio_issues -= 100;
			continue;
		}

		for x in 0..image.width() {
			let pixel = image.get_pixel(x, y);
			let channels = pixel.channels();
			let mut min = (i16::max_value(), 0);

			for (color, id) in COLORS.iter() {
				let (red, green, blue) = *color;
				let red_diff = (channels[0] as i16 - red as i16).abs();
				let green_diff = (channels[1] as i16 - green as i16).abs();
				let blue_diff = (channels[2] as i16 - blue as i16).abs();

				let diff = red_diff + green_diff + blue_diff;
				if diff < min.0 {
					min = (diff, *id);
				}
			}

			result.push_str("\x1b[38;5;");
			result.push_str(min.1.to_string().as_str());
			result.push('m');
			result.push('█');
		}
		result.push('\n');
	}

	result.push_str(COLOR_RESET);
	result
}
pub fn convert_sixel(image: &DynamicImage) -> String {
	let mut data = image.raw_pixels();
	let width = image.width() as i32;
	let height = image.height() as i32;

	let mut output = ptr::null_mut();
	let mut result: Vec<u8> = Vec::new();
	unsafe {
		sixel_sys::sixel_output_new(
			&mut output,
			Some(sixel_output_write),
			&mut result as *mut _ as *mut c_void,
			ptr::null_mut()
		);
	}
	let mut dither = ptr::null_mut();
	unsafe {
		if sixel_sys::sixel_dither_new(&mut dither, 256, ptr::null_mut()) != sixel_sys::OK {
			// TODO: Add way to return an error?
			stderr!("Creating sixel dither failed");
			return String::new();
		}
		if sixel_sys::sixel_dither_initialize(
			dither,
			data.as_mut_ptr(),
			width,
			height,
			sixel_sys::PixelFormat::RGB888,
			sixel_sys::MethodForLargest::Auto,
			sixel_sys::MethodForRepColor::Auto,
			sixel_sys::QualityMode::Auto
		) != sixel_sys::OK
		{
			// 3 = SIXEL_PIXELFORMAT_RGB888
			stderr!("Initializing sixel dither failed");
			return String::new();
		}
		if sixel_sys::sixel_encode(data.as_mut_ptr(), width, height, 1, dither, output) != sixel_sys::OK {
			stderr!("Encoding sixel failed");
			return String::new();
		}
	}

	return String::from_utf8(result).unwrap();
}

unsafe extern "C" fn sixel_output_write(data: *mut c_char, len: c_int, result: *mut c_void) -> i32 {
	(&mut *(result as *mut Vec<u8>))
		.write(slice::from_raw_parts(data as *const c_uchar, len as usize))
		.unwrap();
	0
}

lazy_static! {
	static ref COLORS: HashMap<(u8, u8, u8), u8> = {
		let mut m = HashMap::new();
		m.insert((0,0,0), 0);
		m.insert((128,0,0), 1);
		m.insert((0,128,0), 2);
		m.insert((128,128,0), 3);
		m.insert((0,0,128), 4);
		m.insert((128,0,128), 5);
		m.insert((0,128,128), 6);
		m.insert((192,192,192), 7);
		m.insert((128,128,128), 8);
		m.insert((255,0,0), 9);
		m.insert((0,255,0), 10);
		m.insert((255,255,0), 11);
		m.insert((0,0,255), 12);
		m.insert((255,0,255), 13);
		m.insert((0,255,255), 14);
		m.insert((255,255,255), 15);
		m.insert((0,0,0), 16);
		m.insert((0,0,95), 17);
		m.insert((0,0,135), 18);
		m.insert((0,0,175), 19);
		m.insert((0,0,215), 20);
		m.insert((0,0,255), 21);
		m.insert((0,95,0), 22);
		m.insert((0,95,95), 23);
		m.insert((0,95,135), 24);
		m.insert((0,95,175), 25);
		m.insert((0,95,215), 26);
		m.insert((0,95,255), 27);
		m.insert((0,135,0), 28);
		m.insert((0,135,95), 29);
		m.insert((0,135,135), 30);
		m.insert((0,135,175), 31);
		m.insert((0,135,215), 32);
		m.insert((0,135,255), 33);
		m.insert((0,175,0), 34);
		m.insert((0,175,95), 35);
		m.insert((0,175,135), 36);
		m.insert((0,175,175), 37);
		m.insert((0,175,215), 38);
		m.insert((0,175,255), 39);
		m.insert((0,215,0), 40);
		m.insert((0,215,95), 41);
		m.insert((0,215,135), 42);
		m.insert((0,215,175), 43);
		m.insert((0,215,215), 44);
		m.insert((0,215,255), 45);
		m.insert((0,255,0), 46);
		m.insert((0,255,95), 47);
		m.insert((0,255,135), 48);
		m.insert((0,255,175), 49);
		m.insert((0,255,215), 50);
		m.insert((0,255,255), 51);
		m.insert((95,0,0), 52);
		m.insert((95,0,95), 53);
		m.insert((95,0,135), 54);
		m.insert((95,0,175), 55);
		m.insert((95,0,215), 56);
		m.insert((95,0,255), 57);
		m.insert((95,95,0), 58);
		m.insert((95,95,95), 59);
		m.insert((95,95,135), 60);
		m.insert((95,95,175), 61);
		m.insert((95,95,215), 62);
		m.insert((95,95,255), 63);
		m.insert((95,135,0), 64);
		m.insert((95,135,95), 65);
		m.insert((95,135,135), 66);
		m.insert((95,135,175), 67);
		m.insert((95,135,215), 68);
		m.insert((95,135,255), 69);
		m.insert((95,175,0), 70);
		m.insert((95,175,95), 71);
		m.insert((95,175,135), 72);
		m.insert((95,175,175), 73);
		m.insert((95,175,215), 74);
		m.insert((95,175,255), 75);
		m.insert((95,215,0), 76);
		m.insert((95,215,95), 77);
		m.insert((95,215,135), 78);
		m.insert((95,215,175), 79);
		m.insert((95,215,215), 80);
		m.insert((95,215,255), 81);
		m.insert((95,255,0), 82);
		m.insert((95,255,95), 83);
		m.insert((95,255,135), 84);
		m.insert((95,255,175), 85);
		m.insert((95,255,215), 86);
		m.insert((95,255,255), 87);
		m.insert((135,0,0), 88);
		m.insert((135,0,95), 89);
		m.insert((135,0,135), 90);
		m.insert((135,0,175), 91);
		m.insert((135,0,215), 92);
		m.insert((135,0,255), 93);
		m.insert((135,95,0), 94);
		m.insert((135,95,95), 95);
		m.insert((135,95,135), 96);
		m.insert((135,95,175), 97);
		m.insert((135,95,215), 98);
		m.insert((135,95,255), 99);
		m.insert((135,135,0), 100);
		m.insert((135,135,95), 101);
		m.insert((135,135,135), 102);
		m.insert((135,135,175), 103);
		m.insert((135,135,215), 104);
		m.insert((135,135,255), 105);
		m.insert((135,175,0), 106);
		m.insert((135,175,95), 107);
		m.insert((135,175,135), 108);
		m.insert((135,175,175), 109);
		m.insert((135,175,215), 110);
		m.insert((135,175,255), 111);
		m.insert((135,215,0), 112);
		m.insert((135,215,95), 113);
		m.insert((135,215,135), 114);
		m.insert((135,215,175), 115);
		m.insert((135,215,215), 116);
		m.insert((135,215,255), 117);
		m.insert((135,255,0), 118);
		m.insert((135,255,95), 119);
		m.insert((135,255,135), 120);
		m.insert((135,255,175), 121);
		m.insert((135,255,215), 122);
		m.insert((135,255,255), 123);
		m.insert((175,0,0), 124);
		m.insert((175,0,95), 125);
		m.insert((175,0,135), 126);
		m.insert((175,0,175), 127);
		m.insert((175,0,215), 128);
		m.insert((175,0,255), 129);
		m.insert((175,95,0), 130);
		m.insert((175,95,95), 131);
		m.insert((175,95,135), 132);
		m.insert((175,95,175), 133);
		m.insert((175,95,215), 134);
		m.insert((175,95,255), 135);
		m.insert((175,135,0), 136);
		m.insert((175,135,95), 137);
		m.insert((175,135,135), 138);
		m.insert((175,135,175), 139);
		m.insert((175,135,215), 140);
		m.insert((175,135,255), 141);
		m.insert((175,175,0), 142);
		m.insert((175,175,95), 143);
		m.insert((175,175,135), 144);
		m.insert((175,175,175), 145);
		m.insert((175,175,215), 146);
		m.insert((175,175,255), 147);
		m.insert((175,215,0), 148);
		m.insert((175,215,95), 149);
		m.insert((175,215,135), 150);
		m.insert((175,215,175), 151);
		m.insert((175,215,215), 152);
		m.insert((175,215,255), 153);
		m.insert((175,255,0), 154);
		m.insert((175,255,95), 155);
		m.insert((175,255,135), 156);
		m.insert((175,255,175), 157);
		m.insert((175,255,215), 158);
		m.insert((175,255,255), 159);
		m.insert((215,0,0), 160);
		m.insert((215,0,95), 161);
		m.insert((215,0,135), 162);
		m.insert((215,0,175), 163);
		m.insert((215,0,215), 164);
		m.insert((215,0,255), 165);
		m.insert((215,95,0), 166);
		m.insert((215,95,95), 167);
		m.insert((215,95,135), 168);
		m.insert((215,95,175), 169);
		m.insert((215,95,215), 170);
		m.insert((215,95,255), 171);
		m.insert((215,135,0), 172);
		m.insert((215,135,95), 173);
		m.insert((215,135,135), 174);
		m.insert((215,135,175), 175);
		m.insert((215,135,215), 176);
		m.insert((215,135,255), 177);
		m.insert((215,175,0), 178);
		m.insert((215,175,95), 179);
		m.insert((215,175,135), 180);
		m.insert((215,175,175), 181);
		m.insert((215,175,215), 182);
		m.insert((215,175,255), 183);
		m.insert((215,215,0), 184);
		m.insert((215,215,95), 185);
		m.insert((215,215,135), 186);
		m.insert((215,215,175), 187);
		m.insert((215,215,215), 188);
		m.insert((215,215,255), 189);
		m.insert((215,255,0), 190);
		m.insert((215,255,95), 191);
		m.insert((215,255,135), 192);
		m.insert((215,255,175), 193);
		m.insert((215,255,215), 194);
		m.insert((215,255,255), 195);
		m.insert((255,0,0), 196);
		m.insert((255,0,95), 197);
		m.insert((255,0,135), 198);
		m.insert((255,0,175), 199);
		m.insert((255,0,215), 200);
		m.insert((255,0,255), 201);
		m.insert((255,95,0), 202);
		m.insert((255,95,95), 203);
		m.insert((255,95,135), 204);
		m.insert((255,95,175), 205);
		m.insert((255,95,215), 206);
		m.insert((255,95,255), 207);
		m.insert((255,135,0), 208);
		m.insert((255,135,95), 209);
		m.insert((255,135,135), 210);
		m.insert((255,135,175), 211);
		m.insert((255,135,215), 212);
		m.insert((255,135,255), 213);
		m.insert((255,175,0), 214);
		m.insert((255,175,95), 215);
		m.insert((255,175,135), 216);
		m.insert((255,175,175), 217);
		m.insert((255,175,215), 218);
		m.insert((255,175,255), 219);
		m.insert((255,215,0), 220);
		m.insert((255,215,95), 221);
		m.insert((255,215,135), 222);
		m.insert((255,215,175), 223);
		m.insert((255,215,215), 224);
		m.insert((255,215,255), 225);
		m.insert((255,255,0), 226);
		m.insert((255,255,95), 227);
		m.insert((255,255,135), 228);
		m.insert((255,255,175), 229);
		m.insert((255,255,215), 230);
		m.insert((255,255,255), 231);
		m.insert((8,8,8), 232);
		m.insert((18,18,18), 233);
		m.insert((28,28,28), 234);
		m.insert((38,38,38), 235);
		m.insert((48,48,48), 236);
		m.insert((58,58,58), 237);
		m.insert((68,68,68), 238);
		m.insert((78,78,78), 239);
		m.insert((88,88,88), 240);
		m.insert((98,98,98), 241);
		m.insert((108,108,108), 242);
		m.insert((118,118,118), 243);
		m.insert((128,128,128), 244);
		m.insert((138,138,138), 245);
		m.insert((148,148,148), 246);
		m.insert((158,158,158), 247);
		m.insert((168,168,168), 248);
		m.insert((178,178,178), 249);
		m.insert((188,188,188), 250);
		m.insert((198,198,198), 251);
		m.insert((208,208,208), 252);
		m.insert((218,218,218), 253);
		m.insert((228,228,228), 254);
		m.insert((238,238,238), 255);
		m
	};
}
