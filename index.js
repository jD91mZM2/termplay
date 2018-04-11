window.onload = function() {
    let parallax = document.getElementsByClassName("parallax");
    document.addEventListener("scroll", function(e) {
        for (elem of parallax) {
            elem.style.backgroundPosition = "0 " + (447 - window.scrollY*0.1 - 447) + "px";
        }
    });

    fetch("termplay-wasm/target/wasm32-unknown-unknown/release/termplay.wasm")
        .then(r => r.arrayBuffer())
        .then(r => WebAssembly.instantiate(r))
        .then(termplay => {
            let shellPrompt = "totally-not-fake-bash$ ";

            let terminal = document.getElementById("terminal");
            let xterm = new Terminal();
            xterm.open(terminal);
            xterm.write("Open an image file in the above input!\r\n\n" + shellPrompt);

            let fileTarget = document.getElementById("file").firstElementChild;
            fileTarget.onchange = function(e) {
                let reader = new FileReader();
                reader.onload = function(file) {
                    xterm.write("termplay \"" + e.target.files[0].name + "\"\r\n");

                    let data = new Uint8Array(file.target.result);
                    let len = data.length;

                    let exports = termplay.instance.exports;

                    let slice = exports.slice_new(len);
                    for (let i = 0; i < len; ++i) {
                        exports.slice_set(slice, len, i, data[i]);
                    }
                    let offset = exports.image_to_string(slice, len);

                    let buffer = new Uint8Array(termplay.instance.exports.memory.buffer);

                    if (buffer[offset] != 0) {
                        let string = "";
                        let cursor = offset;
                        while (buffer[cursor] != 0) {
                            string += String.fromCharCode(buffer[cursor]);
                            cursor += 1;
                        }
                        xterm.write(string + "\r\n");
                        exports.free(offset);
                    }
                    xterm.write(shellPrompt);
                };
                reader.readAsArrayBuffer(e.target.files[0]);
            };
        });
};
