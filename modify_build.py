import sys
import re

with open('crates/ffmpeg-sys-next/build.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# 1. Add cfg to extern crate bindgen
content = content.replace('extern crate bindgen;', '#[cfg(feature = "generate-bindings")]\nextern crate bindgen;')

# 2. Add cfg to use bindgen::callbacks
content = content.replace('use bindgen::callbacks::{', '#[cfg(feature = "generate-bindings")]\nuse bindgen::callbacks::{')

# 3. Add cfg to Callbacks struct
content = content.replace('#[derive(Debug)]\nstruct Callbacks;', '#[cfg(feature = "generate-bindings")]\n#[derive(Debug)]\nstruct Callbacks;')

# 4. Add cfg to impl ParseCallbacks
content = content.replace('impl ParseCallbacks for Callbacks {', '#[cfg(feature = "generate-bindings")]\nimpl ParseCallbacks for Callbacks {')

# 5. Wrap the builder block
builder_start = 'let mut builder = bindgen::Builder::default()'
builder_end = 'bindings\n        .write_to_file(output().join("bindings.rs"))\n        .expect("Couldn\'t write bindings!");\n}'

parts = content.split(builder_start)
if len(parts) == 2:
    pre = parts[0]
    rest = builder_start + parts[1]
    
    parts2 = rest.split(builder_end)
    bindgen_logic = parts2[0] + builder_end[:-2] # exclude the last \n}
    
    new_logic = """#[cfg(feature = "generate-bindings")]
    {
        """ + bindgen_logic.replace('\n', '\n        ') + """
    }

    #[cfg(not(feature = "generate-bindings"))]
    {
        let target = env::var("TARGET").unwrap();
        let pregenerated = env::current_dir().unwrap().join(format!("src/bindings_{}.rs", target));
        fs::copy(&pregenerated, output().join("bindings.rs"))
            .expect(&format!("Could not copy pregenerated bindings from {}", pregenerated.display()));
    }
}"""
    
    final_content = pre + new_logic
    
    with open('crates/ffmpeg-sys-next/build.rs', 'w', encoding='utf-8') as f:
        f.write(final_content)
    print("build.rs successfully modified!")
else:
    print("Could not find builder start block")
