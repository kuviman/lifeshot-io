"use strict";

if( typeof Rust === "undefined" ) {
    var Rust = {};
}

(function( root, factory ) {
    if( typeof define === "function" && define.amd ) {
        define( [], factory );
    } else if( typeof module === "object" && module.exports ) {
        module.exports = factory();
    } else {
        Rust.lifeshot_io = factory();
    }
}( this, function() {
    return (function( module_factory ) {
        var instance = module_factory();

        if( typeof process === "object" && typeof process.versions === "object" && typeof process.versions.node === "string" ) {
            var fs = require( "fs" );
            var path = require( "path" );
            var wasm_path = path.join( __dirname, "lifeshot-io.wasm" );
            var buffer = fs.readFileSync( wasm_path );
            var mod = new WebAssembly.Module( buffer );
            var wasm_instance = new WebAssembly.Instance( mod, instance.imports );
            return instance.initialize( wasm_instance );
        } else {
            var file = fetch( "lifeshot-io.wasm", {credentials: "same-origin"} );

            var wasm_instance = ( typeof WebAssembly.instantiateStreaming === "function"
                ? WebAssembly.instantiateStreaming( file, instance.imports )
                    .then( function( result ) { return result.instance; } )

                : file
                    .then( function( response ) { return response.arrayBuffer(); } )
                    .then( function( bytes ) { return WebAssembly.compile( bytes ); } )
                    .then( function( mod ) { return WebAssembly.instantiate( mod, instance.imports ) } ) );

            return wasm_instance
                .then( function( wasm_instance ) {
                    var exports = instance.initialize( wasm_instance );
                    console.log( "Finished loading Rust wasm module 'lifeshot_io'" );
                    return exports;
                })
                .catch( function( error ) {
                    console.log( "Error loading Rust wasm module 'lifeshot_io':", error );
                    throw error;
                });
        }
    }( function() {
    var Module = {};

    

    var HEAP8 = null;
    var HEAP16 = null;
    var HEAP32 = null;
    var HEAPU8 = null;
    var HEAPU16 = null;
    var HEAPU32 = null;
    var HEAPF32 = null;
    var HEAPF64 = null;

    Object.defineProperty( Module, 'exports', { value: {} } );

    function __web_on_grow() {
        var buffer = Module.instance.exports.memory.buffer;
        HEAP8 = new Int8Array( buffer );
        HEAP16 = new Int16Array( buffer );
        HEAP32 = new Int32Array( buffer );
        HEAPU8 = new Uint8Array( buffer );
        HEAPU16 = new Uint16Array( buffer );
        HEAPU32 = new Uint32Array( buffer );
        HEAPF32 = new Float32Array( buffer );
        HEAPF64 = new Float64Array( buffer );
        Module.HEAP8 = HEAP8;
        Module.HEAP16 = HEAP16;
        Module.HEAP32 = HEAP32;
        Module.HEAPU8 = HEAPU8;
        Module.HEAPU16 = HEAPU16;
        Module.HEAPU32 = HEAPU32;
        Module.HEAPF32 = HEAPF32;
        Module.HEAPF64 = HEAPF64;
    }

    return {
        imports: {
            env: {
                
                "__web_on_grow": __web_on_grow
            }
        },
        initialize: function( instance ) {
            Object.defineProperty( Module, 'instance', { value: instance } );
            Object.defineProperty( Module, 'web_malloc', { value: Module.instance.exports.__web_malloc } );
            Object.defineProperty( Module, 'web_free', { value: Module.instance.exports.__web_free } );
            Object.defineProperty( Module, 'web_table', { value: Module.instance.exports.__indirect_function_table } );

            
            __web_on_grow();
            Module.instance.exports.main();

            return Module.exports;
        }
    };
}
 ));
}));
