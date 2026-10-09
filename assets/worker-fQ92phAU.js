/**
* @license
* Copyright 2019 Google LLC
* SPDX-License-Identifier: Apache-2.0
*/
const e=Symbol(`Comlink.proxy`),t=Symbol(`Comlink.endpoint`),n=Symbol(`Comlink.releaseProxy`),r=Symbol(`Comlink.finalizer`),i=Symbol(`Comlink.thrown`),a=e=>typeof e==`object`&&!!e||typeof e==`function`,o=/* @__PURE__ */ new Map([[`proxy`,{canHandle:t=>a(t)&&t[e],serialize(e){let{port1:t,port2:n}=new MessageChannel;return c(e,t),[n,[n]]},deserialize(e){return e.start(),d(e)}}],[`throw`,{canHandle:e=>a(e)&&i in e,serialize({value:e}){let t;return t=e instanceof Error?{isError:!0,value:{message:e.message,name:e.name,stack:e.stack}}:{isError:!1,value:e},[t,[]]},deserialize(e){throw e.isError?Object.assign(Error(e.value.message),e.value):e.value}}]]);function s(e,t){for(let n of e)if(t===n||n===`*`||n instanceof RegExp&&n.test(t))return!0;return!1}function c(e,t=globalThis,n=[`*`]){t.addEventListener(`message`,function a(o){if(!o||!o.data)return;if(!s(n,o.origin)){console.warn(`Invalid origin '${o.origin}' for comlink proxy`);return}let{id:l,type:d,path:f}=Object.assign({path:[]},o.data),p=(o.data.argumentList||[]).map(x),m;try{let t=f.slice(0,-1).reduce((e,t)=>e[t],e),n=f.reduce((e,t)=>e[t],e);switch(d){case`GET`:m=n;break;case`SET`:t[f.slice(-1)[0]]=x(o.data.value),m=!0;break;case`APPLY`:m=n.apply(t,p);break;case`CONSTRUCT`:m=re(new n(...p));break;case`ENDPOINT`:{let{port1:t,port2:n}=new MessageChannel;c(e,n),m=y(t,[t])}break;case`RELEASE`:m=void 0;break;default:return}}catch(e){m={value:e,[i]:0}}Promise.resolve(m).catch(e=>({value:e,[i]:0})).then(n=>{let[i,o]=b(n);t.postMessage(Object.assign(Object.assign({},i),{id:l}),o),d===`RELEASE`&&(t.removeEventListener(`message`,a),u(t),r in e&&typeof e[r]==`function`&&e[r]())}).catch(e=>{let[n,r]=b({value:/* @__PURE__ */ TypeError(`Unserializable return value`),[i]:0});t.postMessage(Object.assign(Object.assign({},n),{id:l}),r)})}),t.start&&t.start()}function l(e){return e.constructor.name===`MessagePort`}function u(e){l(e)&&e.close()}function d(e,t){let n=/* @__PURE__ */ new Map;return e.addEventListener(`message`,function(e){let{data:t}=e;if(!t||!t.id)return;let r=n.get(t.id);if(r)try{r(t)}finally{n.delete(t.id)}}),g(e,n,[],t)}function f(e){if(e)throw Error(`Proxy has been released and is not useable`)}function p(e){return S(e,/* @__PURE__ */ new Map,{type:`RELEASE`}).then(()=>{u(e)})}const m=/* @__PURE__ */ new WeakMap,h=`FinalizationRegistry`in globalThis&&new FinalizationRegistry(e=>{let t=(m.get(e)||0)-1;m.set(e,t),t===0&&p(e)});function ee(e,t){let n=(m.get(t)||0)+1;m.set(t,n),h&&h.register(e,t,e)}function te(e){h&&h.unregister(e)}function g(e,r,i=[],a=function(){}){let o=!1,s=new Proxy(a,{get(t,a){if(f(o),a===n)return()=>{te(s),p(e),r.clear(),o=!0};if(a===`then`){if(i.length===0)return{then:()=>s};let t=S(e,r,{type:`GET`,path:i.map(e=>e.toString())}).then(x);return t.then.bind(t)}return g(e,r,[...i,a])},set(t,n,a){f(o);let[s,c]=b(a);return S(e,r,{type:`SET`,path:[...i,n].map(e=>e.toString()),value:s},c).then(x)},apply(n,a,s){f(o);let c=i[i.length-1];if(c===t)return S(e,r,{type:`ENDPOINT`}).then(x);if(c===`bind`)return g(e,r,i.slice(0,-1));let[l,u]=_(s);return S(e,r,{type:`APPLY`,path:i.map(e=>e.toString()),argumentList:l},u).then(x)},construct(t,n){f(o);let[a,s]=_(n);return S(e,r,{type:`CONSTRUCT`,path:i.map(e=>e.toString()),argumentList:a},s).then(x)}});return ee(s,e),s}function ne(e){return Array.prototype.concat.apply([],e)}function _(e){let t=e.map(b);return[t.map(e=>e[0]),ne(t.map(e=>e[1]))]}const v=/* @__PURE__ */ new WeakMap;function y(e,t){return v.set(e,t),e}function re(t){return Object.assign(t,{[e]:!0})}function b(e){for(let[t,n]of o)if(n.canHandle(e)){let[r,i]=n.serialize(e);return[{type:`HANDLER`,name:t,value:r},i]}return[{type:`RAW`,value:e},v.get(e)||[]]}function x(e){switch(e.type){case`HANDLER`:return o.get(e.name).deserialize(e.value);case`RAW`:return e.value}}function S(e,t,n,r){return new Promise(i=>{let a=ie();t.set(a,i),e.start&&e.start(),e.postMessage(Object.assign({id:a},n),r)})}function ie(){return[,,,,].fill(0).map(()=>Math.floor(Math.random()*(2**53-1)).toString(16)).join(`-`)}var C=class e{static __wrap(t){let n=Object.create(e.prototype);return n.__wbg_ptr=t,T.register(n,n.__wbg_ptr,n),n}__destroy_into_raw(){let e=this.__wbg_ptr;return this.__wbg_ptr=0,T.unregister(this),e}free(){let e=this.__destroy_into_raw();U.__wbg_palette_free(e,0)}static create(e){return U.palette_create(e)}get length(){return U.palette_length(this.__wbg_ptr)>>>0}nearest(e,t){return U.palette_nearest(this.__wbg_ptr,e,t)}resolveSection(e,t){return U.palette_resolveSection(this.__wbg_ptr,e,t)}suggest(e){return U.palette_suggest(this.__wbg_ptr,e)}};Symbol.dispose&&(C.prototype[Symbol.dispose]=C.prototype.free);var w=class e{static __wrap(t){let n=Object.create(e.prototype);return n.__wbg_ptr=t,E.register(n,n.__wbg_ptr,n),n}__destroy_into_raw(){let e=this.__wbg_ptr;return this.__wbg_ptr=0,E.unregister(this),e}free(){let e=this.__destroy_into_raw();U.__wbg_sourceimage_free(e,0)}analyze(e){return U.sourceimage_analyze(this.__wbg_ptr,e)}colorCount(e){return U.sourceimage_colorCount(this.__wbg_ptr,e)}static create(e,t,n){let r=L(e,U.__wbindgen_malloc),i=H;return U.sourceimage_create(r,i,t,n)}get height(){return U.sourceimage_height(this.__wbg_ptr)>>>0}pick(e,t,n,r,i){return ue(i,C),U.sourceimage_pick(this.__wbg_ptr,e,t,I(n)?0:D(n),r,i.__wbg_ptr)}recolor(e,t){var n=L(t,U.__wbindgen_malloc),r=H;return U.sourceimage_recolor(this.__wbg_ptr,e,n,r,t)}get width(){return U.sourceimage_width(this.__wbg_ptr)>>>0}};Symbol.dispose&&(w.prototype[Symbol.dispose]=w.prototype.free);function ae(e,t){return U.composite(e,t)}function oe(e){let t=R(e,U.__wbindgen_malloc,U.__wbindgen_realloc),n=H;return U.parseConfig(t,n)}function se(e){return U.serializeConfig(e)}function ce(e){return U.unprintedColors(e)}function le(){return{__proto__:null,"./rekolor_wasm_bg.js":{__proto__:null,__wbg_Error_30c8987f7c2ed4e2:function(e,t){return Error(M(e,t))},__wbg_Number_14af1003b8dd5ead:function(e){return Number(e)},__wbg_String_8564e559799eccda:function(e,t){let n=R(String(t),U.__wbindgen_malloc,U.__wbindgen_realloc),r=H;j().setInt32(e+4,r,!0),j().setInt32(e+0,n,!0)},__wbg___wbindgen_bigint_get_as_i64_a2383202b9353e4c:function(e,t){let n=t,r=typeof n==`bigint`?n:void 0;j().setBigInt64(e+8,I(r)?BigInt(0):r,!0),j().setInt32(e+0,!I(r),!0)},__wbg___wbindgen_boolean_get_5b446f51afd21013:function(e){let t=e,n=typeof t==`boolean`?t:void 0;return I(n)?16777215:+!!n},__wbg___wbindgen_copy_to_typed_array_88899a52af046901:function(e,t,n){new Uint8Array(n.buffer,n.byteOffset,n.byteLength).set(k(e,t))},__wbg___wbindgen_debug_string_4687d8d8c2017d52:function(e,t){let n=R(O(t),U.__wbindgen_malloc,U.__wbindgen_realloc),r=H;j().setInt32(e+4,r,!0),j().setInt32(e+0,n,!0)},__wbg___wbindgen_in_92f62ee1427d9e49:function(e,t){return e in t},__wbg___wbindgen_is_bigint_b123553bed3bb382:function(e){return typeof e==`bigint`},__wbg___wbindgen_is_function_1f9d30630b8b1d3d:function(e){return typeof e==`function`},__wbg___wbindgen_is_object_3c45d4f2dde4e749:function(e){let t=e;return typeof t==`object`&&!!t},__wbg___wbindgen_is_undefined_8865fb403f8fe9d8:function(e){return e===void 0},__wbg___wbindgen_jsval_eq_02babf21faa37971:function(e,t){return e===t},__wbg___wbindgen_jsval_loose_eq_677f21e468d6b461:function(e,t){return e==t},__wbg___wbindgen_number_get_2e0e7dee9f701a71:function(e,t){let n=t,r=typeof n==`number`?n:void 0;j().setFloat64(e+8,I(r)?0:r,!0),j().setInt32(e+0,!I(r),!0)},__wbg___wbindgen_string_get_0380ccaa2f57f0d9:function(e,t){let n=t,r=typeof n==`string`?n:void 0;var i=I(r)?0:R(r,U.__wbindgen_malloc,U.__wbindgen_realloc),a=H;j().setInt32(e+4,a,!0),j().setInt32(e+0,i,!0)},__wbg___wbindgen_throw_41e9ee4f547fc59a:function(e,t){throw Error(M(e,t))},__wbg_call_6137034ef55c9d0f:function(){return F(function(e,t){return e.call(t)},arguments)},__wbg_debug_52bda05ddf50b736:function(e){console.debug(e)},__wbg_done_b41a1d26cdb37fb6:function(e){return e.done},__wbg_entries_fb6397112b1de25f:function(e){return Object.entries(e)},__wbg_error_757e9472f8410341:function(e,t){let n,r;try{n=e,r=t,console.error(M(e,t))}finally{U.__wbindgen_free(n,r,1)}},__wbg_error_c9cf3fc2064683a9:function(e){console.error(e)},__wbg_get_658f6698067d9515:function(){return F(function(e,t){return Reflect.get(e,t)},arguments)},__wbg_get_6c896e0571ddae51:function(e,t){return e[t>>>0]},__wbg_get_unchecked_288889d017702237:function(e,t){return e[t>>>0]},__wbg_get_with_ref_key_6412cf3094599694:function(e,t){return e[t]},__wbg_info_26925f73a2eee895:function(e){console.info(e)},__wbg_instanceof_ArrayBuffer_a99f175873e5d9b8:function(e){let t;try{t=e instanceof ArrayBuffer}catch{t=!1}return t},__wbg_instanceof_Map_b2611749102d7ba3:function(e){let t;try{t=e instanceof Map}catch{t=!1}return t},__wbg_instanceof_Uint8Array_828cef2aaacafc31:function(e){let t;try{t=e instanceof Uint8Array}catch{t=!1}return t},__wbg_isArray_e15a2ff68ffdbef2:function(e){return Array.isArray(e)},__wbg_isSafeInteger_717808ad6a54bd9e:function(e){return Number.isSafeInteger(e)},__wbg_iterator_e3c31c892080e444:function(){return Symbol.iterator},__wbg_length_7f3c00c40364105e:function(e){return e.length},__wbg_length_d4bdea10311bd9cf:function(e){return e.length},__wbg_log_17c30ef363c61cf4:function(e){console.log(e)},__wbg_new_1dbf7428bba60a42:function(e){return new Uint8Array(e)},__wbg_new_227d7c05414eb861:function(){return/* @__PURE__ */ Error()},__wbg_new_617a8cdb8bb1130e:function(){return{}},__wbg_new_ee2291f50781bf1d:function(){return[]},__wbg_next_33784799010f1bbe:function(e){return e.next},__wbg_next_f4aac29c42af995c:function(){return F(function(e){return e.next()},arguments)},__wbg_palette_new:function(e){return C.__wrap(e)},__wbg_prototypesetcall_bc27214492979395:function(e,t,n){Uint8Array.prototype.set.call(k(e,t),n)},__wbg_set_145a351398b48c65:function(){return F(function(e,t,n){return Reflect.set(e,t,n)},arguments)},__wbg_set_6be42768c690e380:function(e,t,n){e[t]=n},__wbg_set_bea140a88be9b277:function(e,t,n){e[t>>>0]=n},__wbg_sourceimage_new:function(e){return w.__wrap(e)},__wbg_stack_3b0d974bbf31e44f:function(e,t){let n=t.stack,r=R(n,U.__wbindgen_malloc,U.__wbindgen_realloc),i=H;j().setInt32(e+4,i,!0),j().setInt32(e+0,r,!0)},__wbg_value_f3c585ee8f5ba40c:function(e){return e.value},__wbg_warn_13abc63e0d4b3527:function(e){console.warn(e)},__wbindgen_generic_0000000000000001:function(e){return e},__wbindgen_generic_0000000000000002:function(e){return e},__wbindgen_generic_0000000000000003:function(e,t){return M(e,t)},__wbindgen_generic_0000000000000004:function(e){return BigInt.asUintN(64,e)},__wbindgen_init_externref_table:function(){let e=U.__wbindgen_externrefs,t=e.grow(4);e.set(0,void 0),e.set(t+0,void 0),e.set(t+1,null),e.set(t+2,!0),e.set(t+3,!1)}}}}const T=typeof FinalizationRegistry>`u`?{register:()=>{},unregister:()=>{}}:new FinalizationRegistry(e=>U.__wbg_palette_free(e,1)),E=typeof FinalizationRegistry>`u`?{register:()=>{},unregister:()=>{}}:new FinalizationRegistry(e=>U.__wbg_sourceimage_free(e,1));function D(e){let t=U.__externref_table_alloc();return U.__wbindgen_externrefs.set(t,e),t}function ue(e,t){if(!(e instanceof t))throw Error(`expected instance of ${t.name}`)}function O(e){let t=typeof e;if(t==`number`||t==`boolean`||e==null)return`${e}`;if(t==`string`)return`"${e}"`;if(t==`symbol`){let t=e.description;return t==null?`Symbol`:`Symbol(${t})`}if(t==`function`){let t=e.name;return typeof t==`string`&&t.length>0?`Function(${t})`:`Function`}if(Array.isArray(e)){let t=e.length,n=`[`;t>0&&(n+=O(e[0]));for(let r=1;r<t;r++)n+=`, `+O(e[r]);return n+=`]`,n}let n=/\[object ([^\]]+)\]/.exec(toString.call(e)),r;if(n&&n.length>1)r=n[1];else return toString.call(e);if(r==`Object`)try{return`Object(`+JSON.stringify(e)+`)`}catch{return`Object`}return e instanceof Error?`${e.name}: ${e.message}\n${e.stack}`:r}function k(e,t){return e>>>=0,P().subarray(e/1,e/1+t)}let A=null;function j(){return(A===null||A.buffer.detached===!0||A.buffer.detached===void 0&&A.buffer!==U.memory.buffer)&&(A=new DataView(U.memory.buffer)),A}function M(e,t){return de(e>>>0,t)}let N=null;function P(){return(N===null||N.byteLength===0)&&(N=new Uint8Array(U.memory.buffer)),N}function F(e,t){try{return e.apply(this,t)}catch(e){let t=D(e);U.__wbindgen_exn_store(t)}}function I(e){return e==null}function L(e,t){let n=t(e.length*1,1)>>>0;return P().set(e,n/1),H=e.length,n}function R(e,t,n){if(n===void 0){let n=V.encode(e),r=t(n.length,1)>>>0;return P().subarray(r,r+n.length).set(n),H=n.length,r}let r=e.length,i=t(r,1)>>>0,a=P(),o=0;for(;o<r;o++){let t=e.charCodeAt(o);if(t>127)break;a[i+o]=t}if(o!==r){o!==0&&(e=e.slice(o)),i=n(i,r,r=o+e.length*3,1)>>>0;let t=P().subarray(i+o,i+r),a=V.encodeInto(e,t);o+=a.written,i=n(i,r,o,1)>>>0}return H=o,i}let z=new TextDecoder(`utf-8`,{ignoreBOM:!0,fatal:!0});z.decode();let B=0;function de(e,t){return B+=t,B>=2146435072&&(z=new TextDecoder(`utf-8`,{ignoreBOM:!0,fatal:!0}),z.decode(),B=t),z.decode(P().subarray(e,e+t))}const V=new TextEncoder;`encodeInto`in V||(V.encodeInto=function(e,t){let n=V.encode(e);return t.set(n),{read:e.length,written:n.length}});let H=0,U;function fe(e,t){return U=e.exports,A=null,N=null,U.__wbindgen_start(),U}async function pe(e,t){if(typeof Response==`function`&&e instanceof Response){if(!e.ok)throw Error(`failed to fetch Wasm: ${e.status} ${e.statusText} fetching '${e.url}'`);if(typeof WebAssembly.instantiateStreaming==`function`)try{return await WebAssembly.instantiateStreaming(e,t)}catch(t){if(n(e.type)&&e.headers.get(`Content-Type`)!==`application/wasm`)console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n",t);else throw t}let r=await e.arrayBuffer();return await WebAssembly.instantiate(r,t)}{let n=await WebAssembly.instantiate(e,t);return n instanceof WebAssembly.Instance?{instance:n,module:e}:n}function n(e){switch(e){case`basic`:case`cors`:case`default`:return!0}return!1}}async function me(e){if(U!==void 0)return U;e!==void 0&&(Object.getPrototypeOf(e)===Object.prototype?{module_or_path:e}=e:console.warn(`using deprecated parameters for the initialization function; pass a single object instead`)),e===void 0&&(e=new URL(`/reKolor/assets/rekolor_wasm_bg-Buc2lIj0.wasm`,``+import.meta.url));let t=le();(typeof e==`string`||typeof Request==`function`&&e instanceof Request||typeof URL==`function`&&e instanceof URL)&&(e=fetch(e));let{instance:n,module:r}=await pe(await e,t);return fe(n,r)}var he=`/reKolor/assets/rekolor_wasm_bg-Buc2lIj0.wasm`,ge=`{
  "Pure White (non-palette)": {
    "rgb": [255, 255, 255]
  },
  "Pure Black (non-palette)": {
    "rgb": [0, 0, 0]
  },
  "Pantone 100": {
    "rgb": [244, 237, 124]
  },
  "Pantone 101": {
    "rgb": [244, 237, 71]
  },
  "Pantone 102": {
    "rgb": [249, 232, 20]
  },
  "Pantone 103": {
    "rgb": [198, 173, 15]
  },
  "Pantone 104": {
    "rgb": [173, 155, 12]
  },
  "Pantone 105": {
    "rgb": [130, 117, 15]
  },
  "Pantone 106": {
    "rgb": [247, 232, 89]
  },
  "Pantone 107": {
    "rgb": [249, 229, 38]
  },
  "Pantone 108": {
    "rgb": [249, 221, 22]
  },
  "Pantone 109": {
    "rgb": [249, 214, 22]
  },
  "Pantone 110": {
    "rgb": [216, 181, 17]
  },
  "Pantone 111": {
    "rgb": [170, 147, 10]
  },
  "Pantone 112": {
    "rgb": [153, 132, 10]
  },
  "Pantone 113": {
    "rgb": [249, 229, 91]
  },
  "Pantone 114": {
    "rgb": [249, 226, 76]
  },
  "Pantone 115": {
    "rgb": [249, 224, 76]
  },
  "Pantone 116": {
    "rgb": [252, 209, 22]
  },
  "Pantone 117": {
    "rgb": [198, 160, 12]
  },
  "Pantone 118": {
    "rgb": [170, 142, 10]
  },
  "Pantone 119": {
    "rgb": [137, 119, 25]
  },
  "Pantone 120": {
    "rgb": [249, 226, 127]
  },
  "Pantone 121": {
    "rgb": [249, 224, 112]
  },
  "Pantone 122": {
    "rgb": [252, 216, 86]
  },
  "Pantone 123": {
    "rgb": [255, 198, 30]
  },
  "Pantone 124": {
    "rgb": [224, 170, 15]
  },
  "Pantone 125": {
    "rgb": [181, 140, 10]
  },
  "Pantone 126": {
    "rgb": [163, 130, 5]
  },
  "Pantone 127": {
    "rgb": [244, 226, 135]
  },
  "Pantone 128": {
    "rgb": [244, 219, 96]
  },
  "Pantone 129": {
    "rgb": [242, 209, 61]
  },
  "Pantone 130": {
    "rgb": [234, 175, 15]
  },
  "Pantone 131": {
    "rgb": [198, 147, 10]
  },
  "Pantone 132": {
    "rgb": [158, 124, 10]
  },
  "Pantone 133": {
    "rgb": [112, 91, 10]
  },
  "Pantone 134": {
    "rgb": [255, 216, 127]
  },
  "Pantone 135": {
    "rgb": [252, 201, 99]
  },
  "Pantone 136": {
    "rgb": [252, 191, 73]
  },
  "Pantone 137": {
    "rgb": [252, 163, 17]
  },
  "Pantone 138": {
    "rgb": [216, 140, 2]
  },
  "Pantone 139": {
    "rgb": [175, 117, 5]
  },
  "Pantone 140": {
    "rgb": [122, 91, 17]
  },
  "Pantone 141": {
    "rgb": [242, 206, 104]
  },
  "Pantone 142": {
    "rgb": [242, 191, 73]
  },
  "Pantone 143": {
    "rgb": [239, 178, 45]
  },
  "Pantone 144": {
    "rgb": [226, 140, 5]
  },
  "Pantone 145": {
    "rgb": [198, 127, 7]
  },
  "Pantone 146": {
    "rgb": [158, 107, 5]
  },
  "Pantone 147": {
    "rgb": [114, 94, 38]
  },
  "Pantone 148": {
    "rgb": [255, 214, 155]
  },
  "Pantone 149": {
    "rgb": [252, 204, 147]
  },
  "Pantone 150": {
    "rgb": [252, 173, 86]
  },
  "Pantone 151": {
    "rgb": [247, 127, 0]
  },
  "Pantone 152": {
    "rgb": [221, 117, 0]
  },
  "Pantone 153": {
    "rgb": [188, 109, 10]
  },
  "Pantone 154": {
    "rgb": [153, 89, 5]
  },
  "Pantone 155": {
    "rgb": [244, 219, 170]
  },
  "Pantone 156": {
    "rgb": [242, 198, 140]
  },
  "Pantone 157": {
    "rgb": [237, 160, 79]
  },
  "Pantone 158": {
    "rgb": [232, 117, 17]
  },
  "Pantone 159": {
    "rgb": [198, 96, 5]
  },
  "Pantone 160": {
    "rgb": [158, 84, 10]
  },
  "Pantone 161": {
    "rgb": [99, 58, 17]
  },
  "Pantone 162": {
    "rgb": [249, 198, 170]
  },
  "Pantone 163": {
    "rgb": [252, 158, 112]
  },
  "Pantone 164": {
    "rgb": [252, 127, 63]
  },
  "Pantone 165": {
    "rgb": [249, 99, 2]
  },
  "Pantone 166": {
    "rgb": [221, 89, 0]
  },
  "Pantone 167": {
    "rgb": [188, 79, 7]
  },
  "Pantone 168": {
    "rgb": [109, 48, 17]
  },
  "Pantone 169": {
    "rgb": [249, 186, 170]
  },
  "Pantone 170": {
    "rgb": [249, 137, 114]
  },
  "Pantone 171": {
    "rgb": [249, 96, 58]
  },
  "Pantone 172": {
    "rgb": [247, 73, 2]
  },
  "Pantone 173": {
    "rgb": [209, 68, 20]
  },
  "Pantone 174": {
    "rgb": [147, 51, 17]
  },
  "Pantone 175": {
    "rgb": [109, 51, 33]
  },
  "Pantone 176": {
    "rgb": [249, 175, 173]
  },
  "Pantone 177": {
    "rgb": [249, 130, 127]
  },
  "Pantone 178": {
    "rgb": [249, 94, 89]
  },
  "Pantone 179": {
    "rgb": [226, 61, 40]
  },
  "Pantone 180": {
    "rgb": [193, 56, 40]
  },
  "Pantone 181": {
    "rgb": [124, 45, 35]
  },
  "Pantone 182": {
    "rgb": [249, 191, 193]
  },
  "Pantone 183": {
    "rgb": [252, 140, 153]
  },
  "Pantone 184": {
    "rgb": [252, 94, 114]
  },
  "Pantone 185": {
    "rgb": [232, 17, 45]
  },
  "Pantone 186": {
    "rgb": [206, 17, 38]
  },
  "Pantone 187": {
    "rgb": [175, 30, 45]
  },
  "Pantone 188": {
    "rgb": [124, 33, 40]
  },
  "Pantone 189": {
    "rgb": [255, 163, 178]
  },
  "Pantone 190": {
    "rgb": [252, 117, 142]
  },
  "Pantone 191": {
    "rgb": [244, 71, 107]
  },
  "Pantone 192": {
    "rgb": [229, 5, 58]
  },
  "Pantone 193": {
    "rgb": [219, 130, 140]
  },
  "Pantone 194": {
    "rgb": [153, 33, 53]
  },
  "Pantone 196": {
    "rgb": [244, 201, 201]
  },
  "Pantone 197": {
    "rgb": [239, 153, 163]
  },
  "Pantone 198": {
    "rgb": [119, 45, 53]
  },
  "Pantone 199": {
    "rgb": [216, 28, 63]
  },
  "Pantone 200": {
    "rgb": [196, 30, 58]
  },
  "Pantone 201": {
    "rgb": [163, 38, 56]
  },
  "Pantone 202": {
    "rgb": [140, 38, 51]
  },
  "Pantone 203": {
    "rgb": [242, 175, 193]
  },
  "Pantone 204": {
    "rgb": [237, 122, 158]
  },
  "Pantone 205": {
    "rgb": [229, 76, 124]
  },
  "Pantone 206": {
    "rgb": [211, 5, 71]
  },
  "Pantone 207": {
    "rgb": [186, 170, 158]
  },
  "Pantone 208": {
    "rgb": [142, 35, 68]
  },
  "Pantone 209": {
    "rgb": [117, 38, 61]
  },
  "Pantone 210": {
    "rgb": [255, 160, 191]
  },
  "Pantone 211": {
    "rgb": [255, 119, 168]
  },
  "Pantone 212": {
    "rgb": [249, 79, 142]
  },
  "Pantone 213": {
    "rgb": [234, 15, 107]
  },
  "Pantone 214": {
    "rgb": [204, 2, 86]
  },
  "Pantone 215": {
    "rgb": [165, 5, 68]
  },
  "Pantone 216": {
    "rgb": [124, 30, 63]
  },
  "Pantone 217": {
    "rgb": [244, 191, 209]
  },
  "Pantone 218": {
    "rgb": [237, 114, 170]
  },
  "Pantone 219": {
    "rgb": [226, 40, 130]
  },
  "Pantone 220": {
    "rgb": [170, 0, 79]
  },
  "Pantone 221": {
    "rgb": [147, 0, 66]
  },
  "Pantone 222": {
    "rgb": [112, 25, 61]
  },
  "Pantone 223": {
    "rgb": [249, 147, 196]
  },
  "Pantone 224": {
    "rgb": [244, 107, 175]
  },
  "Pantone 225": {
    "rgb": [237, 40, 147]
  },
  "Pantone 226": {
    "rgb": [214, 2, 112]
  },
  "Pantone 227": {
    "rgb": [173, 0, 91]
  },
  "Pantone 228": {
    "rgb": [140, 0, 76]
  },
  "Pantone 229": {
    "rgb": [109, 33, 63]
  },
  "Pantone 230": {
    "rgb": [255, 160, 204]
  },
  "Pantone 231": {
    "rgb": [252, 112, 186]
  },
  "Pantone 232": {
    "rgb": [244, 63, 165]
  },
  "Pantone 233": {
    "rgb": [206, 0, 124]
  },
  "Pantone 234": {
    "rgb": [170, 0, 102]
  },
  "Pantone 235": {
    "rgb": [142, 5, 84]
  },
  "Pantone 236": {
    "rgb": [249, 175, 211]
  },
  "Pantone 237": {
    "rgb": [244, 132, 196]
  },
  "Pantone 238": {
    "rgb": [237, 79, 175]
  },
  "Pantone 239": {
    "rgb": [224, 33, 158]
  },
  "Pantone 240": {
    "rgb": [196, 15, 137]
  },
  "Pantone 241": {
    "rgb": [173, 0, 117]
  },
  "Pantone 242": {
    "rgb": [124, 28, 81]
  },
  "Pantone 243": {
    "rgb": [242, 186, 216]
  },
  "Pantone 244": {
    "rgb": [237, 160, 211]
  },
  "Pantone 245": {
    "rgb": [232, 127, 201]
  },
  "Pantone 246": {
    "rgb": [204, 0, 160]
  },
  "Pantone 247": {
    "rgb": [183, 0, 142]
  },
  "Pantone 248": {
    "rgb": [163, 5, 127]
  },
  "Pantone 249": {
    "rgb": [127, 40, 96]
  },
  "Pantone 250": {
    "rgb": [237, 196, 221]
  },
  "Pantone 251": {
    "rgb": [226, 158, 214]
  },
  "Pantone 252": {
    "rgb": [211, 107, 198]
  },
  "Pantone 253": {
    "rgb": [175, 35, 165]
  },
  "Pantone 254": {
    "rgb": [160, 45, 150]
  },
  "Pantone 255": {
    "rgb": [119, 45, 107]
  },
  "Pantone 256": {
    "rgb": [229, 196, 214]
  },
  "Pantone 257": {
    "rgb": [211, 165, 201]
  },
  "Pantone 258": {
    "rgb": [155, 79, 150]
  },
  "Pantone 259": {
    "rgb": [114, 22, 107]
  },
  "Pantone 260": {
    "rgb": [104, 30, 91]
  },
  "Pantone 261": {
    "rgb": [94, 33, 84]
  },
  "Pantone 262": {
    "rgb": [84, 35, 68]
  },
  "Pantone 263": {
    "rgb": [224, 206, 224]
  },
  "Pantone 264": {
    "rgb": [198, 170, 219]
  },
  "Pantone 265": {
    "rgb": [150, 99, 196]
  },
  "Pantone 266": {
    "rgb": [109, 40, 170]
  },
  "Pantone 267": {
    "rgb": [89, 17, 142]
  },
  "Pantone 268": {
    "rgb": [79, 33, 112]
  },
  "Pantone 269": {
    "rgb": [68, 35, 89]
  },
  "Pantone 270": {
    "rgb": [186, 175, 211]
  },
  "Pantone 271": {
    "rgb": [158, 145, 198]
  },
  "Pantone 272": {
    "rgb": [137, 119, 186]
  },
  "Pantone 273": {
    "rgb": [56, 25, 122]
  },
  "Pantone 274": {
    "rgb": [43, 17, 102]
  },
  "Pantone 275": {
    "rgb": [38, 15, 84]
  },
  "Pantone 276": {
    "rgb": [43, 33, 71]
  },
  "Pantone 277": {
    "rgb": [181, 209, 232]
  },
  "Pantone 278": {
    "rgb": [153, 186, 221]
  },
  "Pantone 279": {
    "rgb": [102, 137, 204]
  },
  "Pantone 280": {
    "rgb": [0, 43, 127]
  },
  "Pantone 281": {
    "rgb": [0, 40, 104]
  },
  "Pantone 282": {
    "rgb": [0, 38, 84]
  },
  "Pantone 283": {
    "rgb": [155, 196, 226]
  },
  "Pantone 284": {
    "rgb": [117, 170, 219]
  },
  "Pantone 285": {
    "rgb": [58, 117, 196]
  },
  "Pantone 286": {
    "rgb": [0, 56, 168]
  },
  "Pantone 287": {
    "rgb": [0, 56, 147]
  },
  "Pantone 288": {
    "rgb": [0, 51, 127]
  },
  "Pantone 289": {
    "rgb": [0, 38, 73]
  },
  "Pantone 290": {
    "rgb": [196, 216, 226]
  },
  "Pantone 291": {
    "rgb": [168, 206, 226]
  },
  "Pantone 292": {
    "rgb": [117, 178, 221]
  },
  "Pantone 293": {
    "rgb": [0, 81, 186]
  },
  "Pantone 294": {
    "rgb": [0, 63, 135]
  },
  "Pantone 295": {
    "rgb": [0, 56, 107]
  },
  "Pantone 296": {
    "rgb": [0, 45, 71]
  },
  "Pantone 297": {
    "rgb": [130, 198, 226]
  },
  "Pantone 298": {
    "rgb": [81, 181, 224]
  },
  "Pantone 299": {
    "rgb": [0, 163, 221]
  },
  "Pantone 300": {
    "rgb": [0, 114, 198]
  },
  "Pantone 301": {
    "rgb": [0, 91, 153]
  },
  "Pantone 302": {
    "rgb": [0, 79, 109]
  },
  "Pantone 303": {
    "rgb": [0, 63, 84]
  },
  "Pantone 304": {
    "rgb": [165, 221, 226]
  },
  "Pantone 305": {
    "rgb": [112, 206, 226]
  },
  "Pantone 306": {
    "rgb": [0, 188, 226]
  },
  "Pantone 307": {
    "rgb": [0, 122, 165]
  },
  "Pantone 308": {
    "rgb": [0, 96, 124]
  },
  "Pantone 309": {
    "rgb": [0, 63, 73]
  },
  "Pantone 310": {
    "rgb": [114, 209, 221]
  },
  "Pantone 311": {
    "rgb": [40, 196, 216]
  },
  "Pantone 312": {
    "rgb": [0, 173, 198]
  },
  "Pantone 313": {
    "rgb": [0, 153, 181]
  },
  "Pantone 314": {
    "rgb": [0, 130, 155]
  },
  "Pantone 315": {
    "rgb": [0, 107, 119]
  },
  "Pantone 316": {
    "rgb": [0, 73, 79]
  },
  "Pantone 317": {
    "rgb": [201, 232, 221]
  },
  "Pantone 318": {
    "rgb": [147, 221, 219]
  },
  "Pantone 319": {
    "rgb": [76, 206, 209]
  },
  "Pantone 320": {
    "rgb": [0, 158, 160]
  },
  "Pantone 321": {
    "rgb": [0, 135, 137]
  },
  "Pantone 322": {
    "rgb": [0, 114, 114]
  },
  "Pantone 323": {
    "rgb": [0, 102, 99]
  },
  "Pantone 324": {
    "rgb": [170, 221, 214]
  },
  "Pantone 325": {
    "rgb": [86, 201, 193]
  },
  "Pantone 326": {
    "rgb": [0, 178, 170]
  },
  "Pantone 327": {
    "rgb": [0, 140, 130]
  },
  "Pantone 328": {
    "rgb": [0, 119, 112]
  },
  "Pantone 329": {
    "rgb": [0, 109, 102]
  },
  "Pantone 330": {
    "rgb": [0, 89, 81]
  },
  "Pantone 331": {
    "rgb": [186, 234, 214]
  },
  "Pantone 332": {
    "rgb": [160, 229, 206]
  },
  "Pantone 333": {
    "rgb": [94, 221, 193]
  },
  "Pantone 334": {
    "rgb": [0, 153, 124]
  },
  "Pantone 335": {
    "rgb": [0, 124, 102]
  },
  "Pantone 336": {
    "rgb": [0, 104, 84]
  },
  "Pantone 337": {
    "rgb": [155, 219, 193]
  },
  "Pantone 338": {
    "rgb": [122, 209, 181]
  },
  "Pantone 339": {
    "rgb": [0, 178, 140]
  },
  "Pantone 340": {
    "rgb": [0, 153, 119]
  },
  "Pantone 341": {
    "rgb": [0, 122, 94]
  },
  "Pantone 342": {
    "rgb": [0, 107, 84]
  },
  "Pantone 343": {
    "rgb": [0, 86, 63]
  },
  "Pantone 344": {
    "rgb": [181, 226, 191]
  },
  "Pantone 345": {
    "rgb": [150, 216, 175]
  },
  "Pantone 346": {
    "rgb": [112, 206, 155]
  },
  "Pantone 347": {
    "rgb": [0, 158, 96]
  },
  "Pantone 348": {
    "rgb": [0, 135, 81]
  },
  "Pantone 349": {
    "rgb": [0, 107, 63]
  },
  "Pantone 350": {
    "rgb": [35, 79, 51]
  },
  "Pantone 351": {
    "rgb": [181, 232, 191]
  },
  "Pantone 352": {
    "rgb": [153, 229, 178]
  },
  "Pantone 353": {
    "rgb": [132, 226, 168]
  },
  "Pantone 354": {
    "rgb": [0, 183, 96]
  },
  "Pantone 355": {
    "rgb": [0, 158, 73]
  },
  "Pantone 356": {
    "rgb": [0, 122, 61]
  },
  "Pantone 357": {
    "rgb": [33, 91, 51]
  },
  "Pantone 358": {
    "rgb": [170, 221, 150]
  },
  "Pantone 359": {
    "rgb": [160, 219, 142]
  },
  "Pantone 360": {
    "rgb": [96, 198, 89]
  },
  "Pantone 361": {
    "rgb": [30, 181, 58]
  },
  "Pantone 362": {
    "rgb": [51, 158, 53]
  },
  "Pantone 363": {
    "rgb": [61, 142, 51]
  },
  "Pantone 364": {
    "rgb": [58, 119, 40]
  },
  "Pantone 365": {
    "rgb": [211, 232, 163]
  },
  "Pantone 366": {
    "rgb": [196, 229, 142]
  },
  "Pantone 367": {
    "rgb": [170, 221, 109]
  },
  "Pantone 368": {
    "rgb": [91, 191, 33]
  },
  "Pantone 369": {
    "rgb": [86, 170, 28]
  },
  "Pantone 370": {
    "rgb": [86, 142, 20]
  },
  "Pantone 371": {
    "rgb": [86, 107, 33]
  },
  "Pantone 372": {
    "rgb": [216, 237, 150]
  },
  "Pantone 373": {
    "rgb": [206, 234, 130]
  },
  "Pantone 374": {
    "rgb": [186, 232, 96]
  },
  "Pantone 375": {
    "rgb": [140, 214, 0]
  },
  "Pantone 376": {
    "rgb": [127, 186, 0]
  },
  "Pantone 377": {
    "rgb": [112, 147, 2]
  },
  "Pantone 378": {
    "rgb": [86, 99, 20]
  },
  "Pantone 379": {
    "rgb": [224, 234, 104]
  },
  "Pantone 380": {
    "rgb": [214, 229, 66]
  },
  "Pantone 381": {
    "rgb": [204, 226, 38]
  },
  "Pantone 382": {
    "rgb": [186, 216, 10]
  },
  "Pantone 383": {
    "rgb": [163, 175, 7]
  },
  "Pantone 384": {
    "rgb": [147, 153, 5]
  },
  "Pantone 385": {
    "rgb": [112, 112, 20]
  },
  "Pantone 386": {
    "rgb": [232, 237, 96]
  },
  "Pantone 387": {
    "rgb": [224, 237, 68]
  },
  "Pantone 388": {
    "rgb": [214, 232, 15]
  },
  "Pantone 389": {
    "rgb": [206, 224, 7]
  },
  "Pantone 390": {
    "rgb": [186, 196, 5]
  },
  "Pantone 391": {
    "rgb": [158, 158, 7]
  },
  "Pantone 392": {
    "rgb": [132, 130, 5]
  },
  "Pantone 393": {
    "rgb": [242, 239, 135]
  },
  "Pantone 394": {
    "rgb": [234, 237, 53]
  },
  "Pantone 395": {
    "rgb": [229, 232, 17]
  },
  "Pantone 396": {
    "rgb": [224, 226, 12]
  },
  "Pantone 397": {
    "rgb": [193, 191, 10]
  },
  "Pantone 398": {
    "rgb": [175, 168, 10]
  },
  "Pantone 399": {
    "rgb": [153, 142, 7]
  },
  "Pantone 400": {
    "rgb": [209, 198, 181]
  },
  "Pantone 401": {
    "rgb": [193, 181, 165]
  },
  "Pantone 402": {
    "rgb": [175, 165, 147]
  },
  "Pantone 403": {
    "rgb": [153, 140, 124]
  },
  "Pantone 404": {
    "rgb": [130, 117, 102]
  },
  "Pantone 405": {
    "rgb": [107, 94, 79]
  },
  "Pantone 406": {
    "rgb": [206, 193, 181]
  },
  "Pantone 408": {
    "rgb": [168, 153, 140]
  },
  "Pantone 409": {
    "rgb": [153, 137, 124]
  },
  "Pantone 410": {
    "rgb": [124, 109, 99]
  },
  "Pantone 411": {
    "rgb": [102, 89, 76]
  },
  "Pantone 412": {
    "rgb": [61, 48, 40]
  },
  "Pantone 413": {
    "rgb": [198, 193, 178]
  },
  "Pantone 414": {
    "rgb": [181, 175, 160]
  },
  "Pantone 415": {
    "rgb": [163, 158, 140]
  },
  "Pantone 416": {
    "rgb": [142, 140, 122]
  },
  "Pantone 417": {
    "rgb": [119, 114, 99]
  },
  "Pantone 418": {
    "rgb": [96, 94, 79]
  },
  "Pantone 419": {
    "rgb": [40, 40, 33]
  },
  "Pantone 420": {
    "rgb": [209, 204, 191]
  },
  "Pantone 421": {
    "rgb": [191, 186, 175]
  },
  "Pantone 422": {
    "rgb": [175, 170, 163]
  },
  "Pantone 423": {
    "rgb": [150, 147, 142]
  },
  "Pantone 424": {
    "rgb": [130, 127, 119]
  },
  "Pantone 425": {
    "rgb": [96, 96, 91]
  },
  "Pantone 426": {
    "rgb": [43, 43, 40]
  },
  "Pantone 427": {
    "rgb": [221, 219, 209]
  },
  "Pantone 428": {
    "rgb": [209, 206, 198]
  },
  "Pantone 429": {
    "rgb": [173, 175, 170]
  },
  "Pantone 430": {
    "rgb": [145, 150, 147]
  },
  "Pantone 431": {
    "rgb": [102, 109, 112]
  },
  "Pantone 432": {
    "rgb": [68, 79, 81]
  },
  "Pantone 433": {
    "rgb": [48, 56, 58]
  },
  "Pantone 434": {
    "rgb": [224, 209, 198]
  },
  "Pantone 435": {
    "rgb": [211, 191, 183]
  },
  "Pantone 436": {
    "rgb": [188, 165, 158]
  },
  "Pantone 437": {
    "rgb": [140, 112, 107]
  },
  "Pantone 438": {
    "rgb": [89, 63, 61]
  },
  "Pantone 439": {
    "rgb": [73, 53, 51]
  },
  "Pantone 440": {
    "rgb": [63, 48, 43]
  },
  "Pantone 441": {
    "rgb": [209, 209, 198]
  },
  "Pantone 442": {
    "rgb": [186, 191, 183]
  },
  "Pantone 443": {
    "rgb": [163, 168, 163]
  },
  "Pantone 444": {
    "rgb": [137, 142, 140]
  },
  "Pantone 445": {
    "rgb": [86, 89, 89]
  },
  "Pantone 446": {
    "rgb": [73, 76, 73]
  },
  "Pantone 447": {
    "rgb": [63, 63, 56]
  },
  "Pantone 448": {
    "rgb": [84, 71, 45]
  },
  "Pantone 449": {
    "rgb": [84, 71, 38]
  },
  "Pantone 450": {
    "rgb": [96, 84, 43]
  },
  "Pantone 451": {
    "rgb": [173, 160, 122]
  },
  "Pantone 452": {
    "rgb": [196, 183, 150]
  },
  "Pantone 453": {
    "rgb": [214, 204, 175]
  },
  "Pantone 454": {
    "rgb": [226, 216, 191]
  },
  "Pantone 455": {
    "rgb": [102, 86, 20]
  },
  "Pantone 456": {
    "rgb": [153, 135, 20]
  },
  "Pantone 457": {
    "rgb": [181, 155, 12]
  },
  "Pantone 458": {
    "rgb": [221, 204, 107]
  },
  "Pantone 459": {
    "rgb": [226, 214, 124]
  },
  "Pantone 460": {
    "rgb": [234, 221, 150]
  },
  "Pantone 461": {
    "rgb": [237, 229, 173]
  },
  "Pantone 462": {
    "rgb": [91, 71, 35]
  },
  "Pantone 463": {
    "rgb": [117, 84, 38]
  },
  "Pantone 464": {
    "rgb": [135, 96, 40]
  },
  "Pantone 465": {
    "rgb": [193, 168, 117]
  },
  "Pantone 466": {
    "rgb": [209, 191, 145]
  },
  "Pantone 467": {
    "rgb": [221, 204, 165]
  },
  "Pantone 468": {
    "rgb": [226, 214, 181]
  },
  "Pantone 469": {
    "rgb": [96, 51, 17]
  },
  "Pantone 470": {
    "rgb": [155, 79, 25]
  },
  "Pantone 471": {
    "rgb": [188, 94, 30]
  },
  "Pantone 472": {
    "rgb": [234, 170, 122]
  },
  "Pantone 473": {
    "rgb": [244, 196, 160]
  },
  "Pantone 474": {
    "rgb": [244, 204, 170]
  },
  "Pantone 475": {
    "rgb": [247, 211, 181]
  },
  "Pantone 476": {
    "rgb": [89, 61, 43]
  },
  "Pantone 477": {
    "rgb": [99, 56, 38]
  },
  "Pantone 478": {
    "rgb": [122, 63, 40]
  },
  "Pantone 479": {
    "rgb": [175, 137, 112]
  },
  "Pantone 480": {
    "rgb": [211, 183, 163]
  },
  "Pantone 481": {
    "rgb": [224, 204, 186]
  },
  "Pantone 482": {
    "rgb": [229, 211, 193]
  },
  "Pantone 483": {
    "rgb": [107, 48, 33]
  },
  "Pantone 484": {
    "rgb": [155, 48, 28]
  },
  "Pantone 485": {
    "rgb": [216, 30, 5]
  },
  "Pantone 486": {
    "rgb": [237, 158, 132]
  },
  "Pantone 487": {
    "rgb": [239, 181, 160]
  },
  "Pantone 488": {
    "rgb": [242, 196, 175]
  },
  "Pantone 489": {
    "rgb": [242, 209, 191]
  },
  "Pantone 490": {
    "rgb": [91, 38, 38]
  },
  "Pantone 491": {
    "rgb": [117, 40, 40]
  },
  "Pantone 492": {
    "rgb": [145, 51, 56]
  },
  "Pantone 494": {
    "rgb": [242, 173, 178]
  },
  "Pantone 495": {
    "rgb": [244, 188, 191]
  },
  "Pantone 496": {
    "rgb": [247, 201, 198]
  },
  "Pantone 497": {
    "rgb": [81, 40, 38]
  },
  "Pantone 498": {
    "rgb": [109, 51, 43]
  },
  "Pantone 499": {
    "rgb": [122, 56, 45]
  },
  "Pantone 500": {
    "rgb": [206, 137, 140]
  },
  "Pantone 501": {
    "rgb": [234, 178, 178]
  },
  "Pantone 502": {
    "rgb": [242, 198, 196]
  },
  "Pantone 503": {
    "rgb": [244, 209, 204]
  },
  "Pantone 504": {
    "rgb": [81, 30, 38]
  },
  "Pantone 505": {
    "rgb": [102, 30, 43]
  },
  "Pantone 506": {
    "rgb": [122, 38, 56]
  },
  "Pantone 507": {
    "rgb": [216, 137, 155]
  },
  "Pantone 508": {
    "rgb": [232, 165, 175]
  },
  "Pantone 509": {
    "rgb": [242, 186, 191]
  },
  "Pantone 510": {
    "rgb": [244, 198, 201]
  },
  "Pantone 511": {
    "rgb": [96, 33, 68]
  },
  "Pantone 512": {
    "rgb": [132, 33, 107]
  },
  "Pantone 513": {
    "rgb": [158, 35, 135]
  },
  "Pantone 514": {
    "rgb": [216, 132, 188]
  },
  "Pantone 515": {
    "rgb": [232, 163, 201]
  },
  "Pantone 516": {
    "rgb": [242, 186, 211]
  },
  "Pantone 517": {
    "rgb": [244, 204, 216]
  },
  "Pantone 518": {
    "rgb": [81, 45, 68]
  },
  "Pantone 519": {
    "rgb": [99, 48, 94]
  },
  "Pantone 520": {
    "rgb": [112, 53, 114]
  },
  "Pantone 521": {
    "rgb": [181, 140, 178]
  },
  "Pantone 522": {
    "rgb": [198, 163, 193]
  },
  "Pantone 523": {
    "rgb": [211, 183, 204]
  },
  "Pantone 524": {
    "rgb": [226, 204, 211]
  },
  "Pantone 525": {
    "rgb": [81, 38, 84]
  },
  "Pantone 526": {
    "rgb": [104, 33, 122]
  },
  "Pantone 527": {
    "rgb": [122, 30, 153]
  },
  "Pantone 528": {
    "rgb": [175, 114, 193]
  },
  "Pantone 529": {
    "rgb": [206, 163, 211]
  },
  "Pantone 530": {
    "rgb": [214, 175, 214]
  },
  "Pantone 531": {
    "rgb": [229, 198, 219]
  },
  "Pantone 532": {
    "rgb": [53, 56, 66]
  },
  "Pantone 533": {
    "rgb": [53, 63, 91]
  },
  "Pantone 534": {
    "rgb": [58, 73, 114]
  },
  "Pantone 535": {
    "rgb": [155, 163, 183]
  },
  "Pantone 536": {
    "rgb": [173, 178, 193]
  },
  "Pantone 537": {
    "rgb": [196, 198, 206]
  },
  "Pantone 538": {
    "rgb": [214, 211, 214]
  },
  "Pantone 539": {
    "rgb": [0, 48, 73]
  },
  "Pantone 540": {
    "rgb": [0, 51, 91]
  },
  "Pantone 541": {
    "rgb": [0, 63, 119]
  },
  "Pantone 542": {
    "rgb": [102, 147, 188]
  },
  "Pantone 543": {
    "rgb": [147, 183, 209]
  },
  "Pantone 544": {
    "rgb": [183, 204, 219]
  },
  "Pantone 545": {
    "rgb": [196, 211, 221]
  },
  "Pantone 546": {
    "rgb": [12, 56, 68]
  },
  "Pantone 547": {
    "rgb": [0, 63, 84]
  },
  "Pantone 548": {
    "rgb": [0, 68, 89]
  },
  "Pantone 549": {
    "rgb": [94, 153, 170]
  },
  "Pantone 550": {
    "rgb": [135, 175, 191]
  },
  "Pantone 551": {
    "rgb": [163, 193, 201]
  },
  "Pantone 552": {
    "rgb": [196, 214, 214]
  },
  "Pantone 553": {
    "rgb": [35, 68, 53]
  },
  "Pantone 554": {
    "rgb": [25, 94, 71]
  },
  "Pantone 555": {
    "rgb": [7, 109, 84]
  },
  "Pantone 556": {
    "rgb": [122, 168, 145]
  },
  "Pantone 557": {
    "rgb": [163, 193, 173]
  },
  "Pantone 558": {
    "rgb": [183, 206, 188]
  },
  "Pantone 559": {
    "rgb": [198, 214, 196]
  },
  "Pantone 560": {
    "rgb": [43, 76, 63]
  },
  "Pantone 561": {
    "rgb": [38, 102, 89]
  },
  "Pantone 562": {
    "rgb": [30, 122, 109]
  },
  "Pantone 563": {
    "rgb": [127, 188, 170]
  },
  "Pantone 564": {
    "rgb": [5, 112, 94]
  },
  "Pantone 565": {
    "rgb": [188, 219, 204]
  },
  "Pantone 566": {
    "rgb": [209, 226, 211]
  },
  "Pantone 567": {
    "rgb": [38, 81, 66]
  },
  "Pantone 569": {
    "rgb": [0, 135, 114]
  },
  "Pantone 570": {
    "rgb": [127, 198, 178]
  },
  "Pantone 571": {
    "rgb": [170, 219, 198]
  },
  "Pantone 572": {
    "rgb": [188, 226, 206]
  },
  "Pantone 573": {
    "rgb": [204, 229, 214]
  },
  "Pantone 574": {
    "rgb": [73, 89, 40]
  },
  "Pantone 575": {
    "rgb": [84, 119, 48]
  },
  "Pantone 576": {
    "rgb": [96, 142, 58]
  },
  "Pantone 577": {
    "rgb": [181, 204, 142]
  },
  "Pantone 578": {
    "rgb": [198, 214, 160]
  },
  "Pantone 579": {
    "rgb": [201, 214, 163]
  },
  "Pantone 580": {
    "rgb": [216, 221, 181]
  },
  "Pantone 581": {
    "rgb": [96, 94, 17]
  },
  "Pantone 582": {
    "rgb": [135, 137, 5]
  },
  "Pantone 583": {
    "rgb": [170, 186, 10]
  },
  "Pantone 584": {
    "rgb": [206, 214, 73]
  },
  "Pantone 585": {
    "rgb": [219, 224, 107]
  },
  "Pantone 586": {
    "rgb": [226, 229, 132]
  },
  "Pantone 587": {
    "rgb": [232, 232, 155]
  },
  "Pantone 600": {
    "rgb": [244, 237, 175]
  },
  "Pantone 601": {
    "rgb": [242, 237, 158]
  },
  "Pantone 602": {
    "rgb": [242, 234, 135]
  },
  "Pantone 603": {
    "rgb": [237, 232, 91]
  },
  "Pantone 604": {
    "rgb": [232, 221, 33]
  },
  "Pantone 605": {
    "rgb": [221, 206, 17]
  },
  "Pantone 606": {
    "rgb": [211, 191, 17]
  },
  "Pantone 607": {
    "rgb": [242, 234, 188]
  },
  "Pantone 608": {
    "rgb": [239, 232, 173]
  },
  "Pantone 609": {
    "rgb": [234, 229, 150]
  },
  "Pantone 610": {
    "rgb": [226, 219, 114]
  },
  "Pantone 611": {
    "rgb": [214, 206, 73]
  },
  "Pantone 612": {
    "rgb": [196, 186, 0]
  },
  "Pantone 613": {
    "rgb": [175, 160, 12]
  },
  "Pantone 614": {
    "rgb": [234, 226, 183]
  },
  "Pantone 615": {
    "rgb": [226, 219, 170]
  },
  "Pantone 616": {
    "rgb": [221, 214, 155]
  },
  "Pantone 617": {
    "rgb": [204, 196, 124]
  },
  "Pantone 618": {
    "rgb": [181, 170, 89]
  },
  "Pantone 619": {
    "rgb": [150, 140, 40]
  },
  "Pantone 620": {
    "rgb": [132, 119, 17]
  },
  "Pantone 621": {
    "rgb": [216, 221, 206]
  },
  "Pantone 622": {
    "rgb": [193, 209, 191]
  },
  "Pantone 623": {
    "rgb": [165, 191, 170]
  },
  "Pantone 624": {
    "rgb": [127, 160, 140]
  },
  "Pantone 625": {
    "rgb": [91, 135, 114]
  },
  "Pantone 626": {
    "rgb": [33, 84, 63]
  },
  "Pantone 627": {
    "rgb": [12, 48, 38]
  },
  "Pantone 628": {
    "rgb": [204, 226, 221]
  },
  "Pantone 629": {
    "rgb": [178, 216, 216]
  },
  "Pantone 630": {
    "rgb": [140, 204, 211]
  },
  "Pantone 631": {
    "rgb": [84, 183, 198]
  },
  "Pantone 632": {
    "rgb": [0, 160, 186]
  },
  "Pantone 633": {
    "rgb": [0, 127, 153]
  },
  "Pantone 634": {
    "rgb": [0, 102, 127]
  },
  "Pantone 635": {
    "rgb": [186, 224, 224]
  },
  "Pantone 636": {
    "rgb": [153, 214, 221]
  },
  "Pantone 637": {
    "rgb": [107, 201, 219]
  },
  "Pantone 638": {
    "rgb": [0, 181, 214]
  },
  "Pantone 639": {
    "rgb": [0, 160, 196]
  },
  "Pantone 640": {
    "rgb": [0, 140, 178]
  },
  "Pantone 641": {
    "rgb": [0, 122, 165]
  },
  "Pantone 642": {
    "rgb": [209, 216, 216]
  },
  "Pantone 643": {
    "rgb": [198, 209, 214]
  },
  "Pantone 644": {
    "rgb": [155, 175, 196]
  },
  "Pantone 645": {
    "rgb": [119, 150, 178]
  },
  "Pantone 646": {
    "rgb": [94, 130, 163]
  },
  "Pantone 647": {
    "rgb": [38, 84, 124]
  },
  "Pantone 648": {
    "rgb": [0, 48, 94]
  },
  "Pantone 649": {
    "rgb": [214, 214, 216]
  },
  "Pantone 650": {
    "rgb": [191, 198, 209]
  },
  "Pantone 651": {
    "rgb": [155, 170, 191]
  },
  "Pantone 652": {
    "rgb": [109, 135, 168]
  },
  "Pantone 653": {
    "rgb": [51, 86, 135]
  },
  "Pantone 654": {
    "rgb": [15, 43, 91]
  },
  "Pantone 655": {
    "rgb": [12, 28, 71]
  },
  "Pantone 656": {
    "rgb": [214, 219, 224]
  },
  "Pantone 657": {
    "rgb": [193, 201, 221]
  },
  "Pantone 658": {
    "rgb": [165, 175, 214]
  },
  "Pantone 659": {
    "rgb": [127, 140, 191]
  },
  "Pantone 660": {
    "rgb": [89, 96, 168]
  },
  "Pantone 661": {
    "rgb": [45, 51, 142]
  },
  "Pantone 662": {
    "rgb": [12, 25, 117]
  },
  "Pantone 663": {
    "rgb": [226, 211, 214]
  },
  "Pantone 664": {
    "rgb": [216, 204, 209]
  },
  "Pantone 665": {
    "rgb": [198, 181, 196]
  },
  "Pantone 666": {
    "rgb": [168, 147, 173]
  },
  "Pantone 667": {
    "rgb": [127, 102, 137]
  },
  "Pantone 668": {
    "rgb": [102, 73, 117]
  },
  "Pantone 669": {
    "rgb": [71, 43, 89]
  },
  "Pantone 670": {
    "rgb": [242, 214, 216]
  },
  "Pantone 671": {
    "rgb": [239, 198, 211]
  },
  "Pantone 672": {
    "rgb": [234, 170, 196]
  },
  "Pantone 673": {
    "rgb": [224, 140, 178]
  },
  "Pantone 674": {
    "rgb": [211, 107, 158]
  },
  "Pantone 675": {
    "rgb": [188, 56, 119]
  },
  "Pantone 676": {
    "rgb": [160, 0, 84]
  },
  "Pantone 677": {
    "rgb": [237, 214, 214]
  },
  "Pantone 678": {
    "rgb": [234, 204, 206]
  },
  "Pantone 679": {
    "rgb": [229, 191, 198]
  },
  "Pantone 680": {
    "rgb": [211, 158, 175]
  },
  "Pantone 681": {
    "rgb": [183, 114, 142]
  },
  "Pantone 682": {
    "rgb": [160, 81, 117]
  },
  "Pantone 683": {
    "rgb": [127, 40, 79]
  },
  "Pantone 684": {
    "rgb": [239, 204, 206]
  },
  "Pantone 685": {
    "rgb": [234, 191, 196]
  },
  "Pantone 686": {
    "rgb": [224, 170, 186]
  },
  "Pantone 687": {
    "rgb": [201, 137, 158]
  },
  "Pantone 688": {
    "rgb": [178, 102, 132]
  },
  "Pantone 689": {
    "rgb": [147, 66, 102]
  },
  "Pantone 690": {
    "rgb": [112, 35, 66]
  },
  "Pantone 691": {
    "rgb": [239, 209, 201]
  },
  "Pantone 692": {
    "rgb": [232, 191, 186]
  },
  "Pantone 693": {
    "rgb": [219, 168, 165]
  },
  "Pantone 694": {
    "rgb": [201, 140, 140]
  },
  "Pantone 695": {
    "rgb": [178, 107, 112]
  },
  "Pantone 696": {
    "rgb": [142, 71, 73]
  },
  "Pantone 697": {
    "rgb": [127, 56, 58]
  },
  "Pantone 698": {
    "rgb": [247, 209, 204]
  },
  "Pantone 699": {
    "rgb": [247, 191, 191]
  },
  "Pantone 700": {
    "rgb": [242, 165, 170]
  },
  "Pantone 701": {
    "rgb": [232, 135, 142]
  },
  "Pantone 702": {
    "rgb": [214, 96, 109]
  },
  "Pantone 703": {
    "rgb": [183, 56, 68]
  },
  "Pantone 704": {
    "rgb": [158, 40, 40]
  },
  "Pantone 705": {
    "rgb": [249, 221, 214]
  },
  "Pantone 706": {
    "rgb": [252, 201, 198]
  },
  "Pantone 707": {
    "rgb": [252, 173, 175]
  },
  "Pantone 708": {
    "rgb": [249, 142, 153]
  },
  "Pantone 709": {
    "rgb": [242, 104, 119]
  },
  "Pantone 710": {
    "rgb": [224, 66, 81]
  },
  "Pantone 711": {
    "rgb": [209, 45, 51]
  },
  "Pantone 712": {
    "rgb": [255, 211, 170]
  },
  "Pantone 713": {
    "rgb": [249, 201, 163]
  },
  "Pantone 714": {
    "rgb": [249, 186, 130]
  },
  "Pantone 715": {
    "rgb": [252, 158, 73]
  },
  "Pantone 716": {
    "rgb": [242, 132, 17]
  },
  "Pantone 717": {
    "rgb": [211, 109, 0]
  },
  "Pantone 718": {
    "rgb": [191, 91, 0]
  },
  "Pantone 719": {
    "rgb": [244, 209, 175]
  },
  "Pantone 720": {
    "rgb": [239, 196, 158]
  },
  "Pantone 721": {
    "rgb": [232, 178, 130]
  },
  "Pantone 722": {
    "rgb": [209, 142, 84]
  },
  "Pantone 723": {
    "rgb": [186, 117, 48]
  },
  "Pantone 724": {
    "rgb": [142, 73, 5]
  },
  "Pantone 725": {
    "rgb": [117, 56, 2]
  },
  "Pantone 726": {
    "rgb": [237, 211, 181]
  },
  "Pantone 727": {
    "rgb": [226, 191, 155]
  },
  "Pantone 728": {
    "rgb": [211, 168, 124]
  },
  "Pantone 729": {
    "rgb": [193, 142, 96]
  },
  "Pantone 730": {
    "rgb": [170, 117, 63]
  },
  "Pantone 731": {
    "rgb": [114, 63, 10]
  },
  "Pantone 732": {
    "rgb": [96, 51, 10]
  },
  "Pantone 801": {
    "rgb": [0, 170, 204]
  },
  "Pantone 802": {
    "rgb": [96, 221, 73]
  },
  "Pantone 803": {
    "rgb": [255, 237, 56]
  },
  "Pantone 804": {
    "rgb": [255, 147, 56]
  },
  "Pantone 805": {
    "rgb": [249, 89, 81]
  },
  "Pantone 806": {
    "rgb": [255, 0, 147]
  },
  "Pantone 807": {
    "rgb": [214, 0, 158]
  },
  "Pantone 808": {
    "rgb": [0, 181, 155]
  },
  "Pantone 809": {
    "rgb": [221, 224, 15]
  },
  "Pantone 810": {
    "rgb": [255, 204, 30]
  },
  "Pantone 811": {
    "rgb": [255, 114, 71]
  },
  "Pantone 812": {
    "rgb": [252, 35, 102]
  },
  "Pantone 813": {
    "rgb": [229, 0, 153]
  },
  "Pantone 814": {
    "rgb": [140, 96, 193]
  },
  "Pantone 1205": {
    "rgb": [247, 232, 170]
  },
  "Pantone 1215": {
    "rgb": [249, 224, 140]
  },
  "Pantone 1225": {
    "rgb": [255, 204, 73]
  },
  "Pantone 1235": {
    "rgb": [252, 181, 20]
  },
  "Pantone 1245": {
    "rgb": [191, 145, 12]
  },
  "Pantone 1255": {
    "rgb": [163, 127, 20]
  },
  "Pantone 1265": {
    "rgb": [124, 99, 22]
  },
  "Pantone 1345": {
    "rgb": [255, 214, 145]
  },
  "Pantone 1355": {
    "rgb": [252, 206, 135]
  },
  "Pantone 1365": {
    "rgb": [252, 186, 94]
  },
  "Pantone 1375": {
    "rgb": [249, 155, 12]
  },
  "Pantone 1385": {
    "rgb": [204, 122, 2]
  },
  "Pantone 1395": {
    "rgb": [153, 96, 7]
  },
  "Pantone 1405": {
    "rgb": [107, 71, 20]
  },
  "Pantone 1485": {
    "rgb": [255, 183, 119]
  },
  "Pantone 1495": {
    "rgb": [255, 153, 63]
  },
  "Pantone 1505": {
    "rgb": [244, 124, 0]
  },
  "Pantone 1525": {
    "rgb": [181, 84, 0]
  },
  "Pantone 1535": {
    "rgb": [140, 68, 0]
  },
  "Pantone 1545": {
    "rgb": [76, 40, 15]
  },
  "Pantone 1555": {
    "rgb": [249, 191, 158]
  },
  "Pantone 1565": {
    "rgb": [252, 165, 119]
  },
  "Pantone 1575": {
    "rgb": [252, 135, 68]
  },
  "Pantone 1585": {
    "rgb": [249, 107, 7]
  },
  "Pantone 1595": {
    "rgb": [209, 91, 5]
  },
  "Pantone 1605": {
    "rgb": [160, 79, 17]
  },
  "Pantone 1615": {
    "rgb": [132, 63, 15]
  },
  "Pantone 1625": {
    "rgb": [249, 165, 140]
  },
  "Pantone 1635": {
    "rgb": [249, 142, 109]
  },
  "Pantone 1645": {
    "rgb": [249, 114, 66]
  },
  "Pantone 1655": {
    "rgb": [249, 86, 2]
  },
  "Pantone 1665": {
    "rgb": [221, 79, 5]
  },
  "Pantone 1675": {
    "rgb": [165, 63, 15]
  },
  "Pantone 1685": {
    "rgb": [132, 53, 17]
  },
  "Pantone 1765": {
    "rgb": [249, 158, 163]
  },
  "Pantone 1767": {
    "rgb": [249, 178, 183]
  },
  "Pantone 1775": {
    "rgb": [249, 132, 142]
  },
  "Pantone 1777": {
    "rgb": [252, 102, 117]
  },
  "Pantone 1785": {
    "rgb": [252, 79, 89]
  },
  "Pantone 1787": {
    "rgb": [244, 63, 79]
  },
  "Pantone 1788": {
    "rgb": [239, 43, 45]
  },
  "Pantone 1795": {
    "rgb": [214, 40, 40]
  },
  "Pantone 1797": {
    "rgb": [204, 45, 48]
  },
  "Pantone 1805": {
    "rgb": [175, 38, 38]
  },
  "Pantone 1807": {
    "rgb": [160, 48, 51]
  },
  "Pantone 1810": {
    "rgb": [124, 33, 30]
  },
  "Pantone 1817": {
    "rgb": [91, 45, 40]
  },
  "Pantone 1895": {
    "rgb": [252, 191, 201]
  },
  "Pantone 1905": {
    "rgb": [252, 155, 178]
  },
  "Pantone 1915": {
    "rgb": [244, 84, 124]
  },
  "Pantone 1925": {
    "rgb": [224, 7, 71]
  },
  "Pantone 1935": {
    "rgb": [193, 5, 56]
  },
  "Pantone 1945": {
    "rgb": [168, 12, 53]
  },
  "Pantone 1955": {
    "rgb": [147, 22, 56]
  },
  "Pantone 2365": {
    "rgb": [247, 196, 216]
  },
  "Pantone 2375": {
    "rgb": [234, 107, 191]
  },
  "Pantone 2385": {
    "rgb": [219, 40, 165]
  },
  "Pantone 2395": {
    "rgb": [196, 0, 140]
  },
  "Pantone 2405": {
    "rgb": [168, 0, 122]
  },
  "Pantone 2415": {
    "rgb": [155, 0, 112]
  },
  "Pantone 2425": {
    "rgb": [135, 0, 91]
  },
  "Pantone 2562": {
    "rgb": [216, 168, 216]
  },
  "Pantone 2563": {
    "rgb": [209, 160, 204]
  },
  "Pantone 2567": {
    "rgb": [191, 147, 204]
  },
  "Pantone 2572": {
    "rgb": [198, 135, 209]
  },
  "Pantone 2573": {
    "rgb": [186, 124, 188]
  },
  "Pantone 2577": {
    "rgb": [170, 114, 191]
  },
  "Pantone 2582": {
    "rgb": [170, 71, 186]
  },
  "Pantone 2583": {
    "rgb": [158, 79, 165]
  },
  "Pantone 2587": {
    "rgb": [142, 71, 173]
  },
  "Pantone 2592": {
    "rgb": [147, 15, 165]
  },
  "Pantone 2593": {
    "rgb": [135, 43, 147]
  },
  "Pantone 2597": {
    "rgb": [102, 0, 140]
  },
  "Pantone 2602": {
    "rgb": [130, 12, 142]
  },
  "Pantone 2603": {
    "rgb": [112, 20, 122]
  },
  "Pantone 2607": {
    "rgb": [91, 2, 122]
  },
  "Pantone 2612": {
    "rgb": [112, 30, 114]
  },
  "Pantone 2613": {
    "rgb": [102, 17, 109]
  },
  "Pantone 2617": {
    "rgb": [86, 12, 112]
  },
  "Pantone 2622": {
    "rgb": [96, 45, 89]
  },
  "Pantone 2623": {
    "rgb": [91, 25, 94]
  },
  "Pantone 2627": {
    "rgb": [76, 20, 94]
  },
  "Pantone 2635": {
    "rgb": [201, 173, 216]
  },
  "Pantone 2645": {
    "rgb": [181, 145, 209]
  },
  "Pantone 2655": {
    "rgb": [155, 109, 198]
  },
  "Pantone 2665": {
    "rgb": [137, 79, 191]
  },
  "Pantone 2685": {
    "rgb": [86, 0, 140]
  },
  "Pantone 2695": {
    "rgb": [68, 35, 94]
  },
  "Pantone 2705": {
    "rgb": [173, 158, 211]
  },
  "Pantone 2706": {
    "rgb": [209, 206, 221]
  },
  "Pantone 2707": {
    "rgb": [191, 209, 229]
  },
  "Pantone 2708": {
    "rgb": [175, 188, 219]
  },
  "Pantone 2715": {
    "rgb": [147, 122, 204]
  },
  "Pantone 2716": {
    "rgb": [165, 160, 214]
  },
  "Pantone 2717": {
    "rgb": [165, 186, 224]
  },
  "Pantone 2718": {
    "rgb": [91, 119, 204]
  },
  "Pantone 2725": {
    "rgb": [114, 81, 188]
  },
  "Pantone 2726": {
    "rgb": [102, 86, 188]
  },
  "Pantone 2727": {
    "rgb": [94, 104, 196]
  },
  "Pantone 2728": {
    "rgb": [48, 68, 181]
  },
  "Pantone 2735": {
    "rgb": [79, 0, 147]
  },
  "Pantone 2736": {
    "rgb": [73, 48, 173]
  },
  "Pantone 2738": {
    "rgb": [45, 0, 142]
  },
  "Pantone 2745": {
    "rgb": [63, 0, 119]
  },
  "Pantone 2746": {
    "rgb": [63, 40, 147]
  },
  "Pantone 2747": {
    "rgb": [28, 20, 107]
  },
  "Pantone 2748": {
    "rgb": [30, 28, 119]
  },
  "Pantone 2755": {
    "rgb": [53, 0, 109]
  },
  "Pantone 2756": {
    "rgb": [51, 40, 117]
  },
  "Pantone 2757": {
    "rgb": [20, 22, 84]
  },
  "Pantone 2758": {
    "rgb": [25, 33, 104]
  },
  "Pantone 2765": {
    "rgb": [43, 12, 86]
  },
  "Pantone 2766": {
    "rgb": [43, 38, 91]
  },
  "Pantone 2767": {
    "rgb": [20, 33, 61]
  },
  "Pantone 2768": {
    "rgb": [17, 33, 81]
  },
  "Pantone 2905": {
    "rgb": [147, 198, 224]
  },
  "Pantone 2915": {
    "rgb": [96, 175, 221]
  },
  "Pantone 2925": {
    "rgb": [0, 142, 214]
  },
  "Pantone 2935": {
    "rgb": [0, 91, 191]
  },
  "Pantone 2945": {
    "rgb": [0, 84, 160]
  },
  "Pantone 2955": {
    "rgb": [0, 61, 107]
  },
  "Pantone 2965": {
    "rgb": [0, 51, 76]
  },
  "Pantone 2975": {
    "rgb": [186, 224, 226]
  },
  "Pantone 2985": {
    "rgb": [81, 191, 226]
  },
  "Pantone 2995": {
    "rgb": [0, 165, 219]
  },
  "Pantone 3005": {
    "rgb": [0, 132, 201]
  },
  "Pantone 3015": {
    "rgb": [0, 112, 158]
  },
  "Pantone 3025": {
    "rgb": [0, 84, 107]
  },
  "Pantone 3035": {
    "rgb": [0, 68, 84]
  },
  "Pantone 3105": {
    "rgb": [127, 214, 219]
  },
  "Pantone 3115": {
    "rgb": [45, 198, 214]
  },
  "Pantone 3125": {
    "rgb": [0, 183, 198]
  },
  "Pantone 3135": {
    "rgb": [0, 155, 170]
  },
  "Pantone 3145": {
    "rgb": [0, 132, 142]
  },
  "Pantone 3155": {
    "rgb": [0, 109, 117]
  },
  "Pantone 3165": {
    "rgb": [0, 86, 91]
  },
  "Pantone 3242": {
    "rgb": [135, 221, 209]
  },
  "Pantone 3245": {
    "rgb": [140, 224, 209]
  },
  "Pantone 3248": {
    "rgb": [122, 211, 193]
  },
  "Pantone 3252": {
    "rgb": [86, 214, 201]
  },
  "Pantone 3255": {
    "rgb": [71, 214, 193]
  },
  "Pantone 3258": {
    "rgb": [53, 196, 175]
  },
  "Pantone 3262": {
    "rgb": [0, 193, 181]
  },
  "Pantone 3265": {
    "rgb": [0, 198, 178]
  },
  "Pantone 3268": {
    "rgb": [0, 175, 153]
  },
  "Pantone 3272": {
    "rgb": [0, 170, 158]
  },
  "Pantone 3275": {
    "rgb": [0, 178, 160]
  },
  "Pantone 3278": {
    "rgb": [0, 155, 132]
  },
  "Pantone 3282": {
    "rgb": [0, 140, 130]
  },
  "Pantone 3285": {
    "rgb": [0, 153, 135]
  },
  "Pantone 3288": {
    "rgb": [0, 130, 112]
  },
  "Pantone 3292": {
    "rgb": [0, 96, 86]
  },
  "Pantone 3295": {
    "rgb": [0, 130, 114]
  },
  "Pantone 3298": {
    "rgb": [0, 107, 91]
  },
  "Pantone 3302": {
    "rgb": [0, 73, 63]
  },
  "Pantone 3305": {
    "rgb": [0, 79, 66]
  },
  "Pantone 3308": {
    "rgb": [0, 68, 56]
  },
  "Pantone 3375": {
    "rgb": [142, 226, 188]
  },
  "Pantone 3385": {
    "rgb": [84, 216, 168]
  },
  "Pantone 3395": {
    "rgb": [0, 201, 147]
  },
  "Pantone 3405": {
    "rgb": [0, 178, 122]
  },
  "Pantone 3415": {
    "rgb": [0, 124, 89]
  },
  "Pantone 3425": {
    "rgb": [0, 104, 71]
  },
  "Pantone 3435": {
    "rgb": [2, 73, 48]
  },
  "Pantone 3935": {
    "rgb": [242, 237, 109]
  },
  "Pantone 3945": {
    "rgb": [239, 234, 7]
  },
  "Pantone 3955": {
    "rgb": [237, 226, 17]
  },
  "Pantone 3965": {
    "rgb": [232, 221, 17]
  },
  "Pantone 3975": {
    "rgb": [181, 168, 12]
  },
  "Pantone 3985": {
    "rgb": [153, 140, 10]
  },
  "Pantone 3995": {
    "rgb": [109, 96, 2]
  },
  "Pantone 4485": {
    "rgb": [96, 76, 17]
  },
  "Pantone 4495": {
    "rgb": [135, 117, 48]
  },
  "Pantone 4505": {
    "rgb": [160, 145, 81]
  },
  "Pantone 4515": {
    "rgb": [188, 173, 117]
  },
  "Pantone 4525": {
    "rgb": [204, 191, 142]
  },
  "Pantone 4535": {
    "rgb": [219, 206, 165]
  },
  "Pantone 4545": {
    "rgb": [229, 219, 186]
  },
  "Pantone 4625": {
    "rgb": [71, 35, 17]
  },
  "Pantone 4635": {
    "rgb": [140, 89, 51]
  },
  "Pantone 4645": {
    "rgb": [178, 130, 96]
  },
  "Pantone 4655": {
    "rgb": [196, 153, 119]
  },
  "Pantone 4665": {
    "rgb": [216, 181, 150]
  },
  "Pantone 4675": {
    "rgb": [229, 198, 170]
  },
  "Pantone 4685": {
    "rgb": [237, 211, 188]
  },
  "Pantone 4695": {
    "rgb": [81, 38, 28]
  },
  "Pantone 4705": {
    "rgb": [124, 81, 61]
  },
  "Pantone 4715": {
    "rgb": [153, 112, 91]
  },
  "Pantone 4725": {
    "rgb": [181, 145, 124]
  },
  "Pantone 4735": {
    "rgb": [204, 175, 155]
  },
  "Pantone 4745": {
    "rgb": [216, 191, 170]
  },
  "Pantone 4755": {
    "rgb": [226, 204, 186]
  },
  "Pantone 4975": {
    "rgb": [68, 30, 28]
  },
  "Pantone 4985": {
    "rgb": [132, 73, 73]
  },
  "Pantone 4995": {
    "rgb": [165, 107, 109]
  },
  "Pantone 5005": {
    "rgb": [188, 135, 135]
  },
  "Pantone 5015": {
    "rgb": [216, 173, 168]
  },
  "Pantone 5025": {
    "rgb": [226, 188, 183]
  },
  "Pantone 5035": {
    "rgb": [237, 206, 198]
  },
  "Pantone 5115": {
    "rgb": [79, 33, 58]
  },
  "Pantone 5125": {
    "rgb": [117, 71, 96]
  },
  "Pantone 5135": {
    "rgb": [147, 107, 127]
  },
  "Pantone 5145": {
    "rgb": [173, 135, 153]
  },
  "Pantone 5155": {
    "rgb": [204, 175, 183]
  },
  "Pantone 5165": {
    "rgb": [224, 201, 204]
  },
  "Pantone 5175": {
    "rgb": [232, 214, 209]
  },
  "Pantone 5185": {
    "rgb": [71, 40, 53]
  },
  "Pantone 5195": {
    "rgb": [89, 51, 68]
  },
  "Pantone 5205": {
    "rgb": [142, 104, 119]
  },
  "Pantone 5215": {
    "rgb": [181, 147, 155]
  },
  "Pantone 5225": {
    "rgb": [204, 173, 175]
  },
  "Pantone 5235": {
    "rgb": [221, 198, 196]
  },
  "Pantone 5245": {
    "rgb": [229, 211, 204]
  },
  "Pantone 5255": {
    "rgb": [53, 38, 79]
  },
  "Pantone 5265": {
    "rgb": [73, 61, 99]
  },
  "Pantone 5275": {
    "rgb": [96, 86, 119]
  },
  "Pantone 5285": {
    "rgb": [140, 130, 153]
  },
  "Pantone 5295": {
    "rgb": [178, 168, 181]
  },
  "Pantone 5305": {
    "rgb": [204, 193, 198]
  },
  "Pantone 5315": {
    "rgb": [219, 211, 211]
  },
  "Pantone 5395": {
    "rgb": [2, 40, 58]
  },
  "Pantone 5405": {
    "rgb": [63, 96, 117]
  },
  "Pantone 5415": {
    "rgb": [96, 124, 140]
  },
  "Pantone 5425": {
    "rgb": [132, 153, 165]
  },
  "Pantone 5435": {
    "rgb": [175, 188, 191]
  },
  "Pantone 5445": {
    "rgb": [196, 204, 204]
  },
  "Pantone 5455": {
    "rgb": [214, 216, 211]
  },
  "Pantone 5463": {
    "rgb": [0, 53, 58]
  },
  "Pantone 5467": {
    "rgb": [25, 56, 51]
  },
  "Pantone 5473": {
    "rgb": [38, 104, 109]
  },
  "Pantone 5477": {
    "rgb": [58, 86, 79]
  },
  "Pantone 5483": {
    "rgb": [96, 145, 145]
  },
  "Pantone 5487": {
    "rgb": [102, 124, 114]
  },
  "Pantone 5493": {
    "rgb": [140, 175, 173]
  },
  "Pantone 5497": {
    "rgb": [145, 163, 153]
  },
  "Pantone 5503": {
    "rgb": [170, 196, 191]
  },
  "Pantone 5507": {
    "rgb": [175, 186, 178]
  },
  "Pantone 5513": {
    "rgb": [206, 216, 209]
  },
  "Pantone 5517": {
    "rgb": [201, 206, 196]
  },
  "Pantone 5523": {
    "rgb": [214, 221, 214]
  },
  "Pantone 5527": {
    "rgb": [206, 209, 198]
  },
  "Pantone 5535": {
    "rgb": [33, 61, 48]
  },
  "Pantone 5545": {
    "rgb": [79, 109, 94]
  },
  "Pantone 5555": {
    "rgb": [119, 145, 130]
  },
  "Pantone 5565": {
    "rgb": [150, 170, 153]
  },
  "Pantone 5575": {
    "rgb": [175, 191, 173]
  },
  "Pantone 5585": {
    "rgb": [196, 206, 191]
  },
  "Pantone 5595": {
    "rgb": [216, 219, 204]
  },
  "Pantone 5605": {
    "rgb": [35, 58, 45]
  },
  "Pantone 5615": {
    "rgb": [84, 104, 86]
  },
  "Pantone 5625": {
    "rgb": [114, 132, 112]
  },
  "Pantone 5635": {
    "rgb": [158, 170, 153]
  },
  "Pantone 5645": {
    "rgb": [188, 193, 178]
  },
  "Pantone 5655": {
    "rgb": [198, 204, 186]
  },
  "Pantone 5665": {
    "rgb": [214, 214, 198]
  },
  "Pantone 5743": {
    "rgb": [63, 73, 38]
  },
  "Pantone 5747": {
    "rgb": [66, 71, 22]
  },
  "Pantone 5753": {
    "rgb": [94, 102, 58]
  },
  "Pantone 5757": {
    "rgb": [107, 112, 43]
  },
  "Pantone 5763": {
    "rgb": [119, 124, 79]
  },
  "Pantone 5767": {
    "rgb": [140, 145, 79]
  },
  "Pantone 5773": {
    "rgb": [155, 158, 114]
  },
  "Pantone 5777": {
    "rgb": [170, 173, 117]
  },
  "Pantone 5783": {
    "rgb": [181, 181, 142]
  },
  "Pantone 5787": {
    "rgb": [198, 198, 153]
  },
  "Pantone 5793": {
    "rgb": [198, 198, 165]
  },
  "Pantone 5797": {
    "rgb": [211, 209, 170]
  },
  "Pantone 5803": {
    "rgb": [216, 214, 183]
  },
  "Pantone 5807": {
    "rgb": [224, 221, 188]
  },
  "Pantone 5815": {
    "rgb": [73, 68, 17]
  },
  "Pantone 5825": {
    "rgb": [117, 112, 43]
  },
  "Pantone 5835": {
    "rgb": [158, 153, 89]
  },
  "Pantone 5845": {
    "rgb": [178, 170, 112]
  },
  "Pantone 5855": {
    "rgb": [204, 198, 147]
  },
  "Pantone 5865": {
    "rgb": [214, 206, 163]
  },
  "Pantone 5875": {
    "rgb": [224, 219, 181]
  }
}
`;const W={fileBytes:52428800,side:16384,pixels:24e6,configBytes:262144,picks:256,unprinted:256};function G(e,t){if(!(e>0&&t>0))return`the image is empty (${e}×${t})`;if(e>W.side||t>W.side)return`the image is ${e}×${t}; each side can be at most ${W.side} pixels`;let n=e*t;if(n>W.pixels){let e=e=>(e/1e6).toFixed(1);return`the image has ${e(n)} megapixels; at most ${e(W.pixels)} are supported`}}function _e(e){if(e>W.fileBytes){let t=e=>(e/1024/1024).toFixed(1);return`the file is ${t(e)} MB; at most ${t(W.fileBytes)} MB is supported`}}const K=e=>({status:`ok`,value:e}),q=(e,t)=>({status:`error`,error:{kind:e,message:t}});async function ve(e){let t=_e(e.size);if(t)return q(`fileTooLarge`,t);let n;try{n=await createImageBitmap(e,{imageOrientation:`from-image`,colorSpaceConversion:`default`})}catch(e){return q(`decodeFailed`,`the browser couldn't decode this file (${e.message})`)}let{width:r,height:i}=n,a=G(r,i);if(a)return n.close(),q(`imageTooLarge`,a);try{let e=new OffscreenCanvas(r,i).getContext(`2d`,{willReadFrequently:!0});if(!e)throw Error(`no 2D canvas context`);e.drawImage(n,0,0);let t=e.getImageData(0,0,r,i,{colorSpace:`srgb`}).data;return K({rgba:new Uint8Array(t.buffer),width:r,height:i,bitmap:n})}catch(e){return n.close(),q(`decodeFailed`,`reading the decoded pixels failed (${e.message})`)}}const J=(e,t,n)=>new ImageData(new Uint8ClampedArray(e.buffer,e.byteOffset,e.byteLength),t,n);function ye(e,t,n){return createImageBitmap(J(e,t,n))}function be(e,t,n){let r=new OffscreenCanvas(t,n),i=r.getContext(`2d`);if(!i)throw Error(`no 2D canvas context`);return i.putImageData(J(e,t,n),0,0),r.convertToBlob({type:`image/png`})}function xe(e){let t=JSON.parse(e);if(typeof t!=`object`||!t||Array.isArray(t))throw Error(`palette JSON must be an object of { rgb: [r, g, b] } entries`);return{entries:Object.entries(t).map(([e,t])=>{let n=t.rgb;if(!Array.isArray(n)||n.length!==3||!n.every(e=>Number.isInteger(e)&&e>=0&&e<=255))throw Error(`palette entry ${JSON.stringify(e)}: rgb must be three integers 0–255`);let[r,i,a]=n;return{name:e,rgb:{r,g:i,b:a}}})}}const Y=e=>e.status===`ok`?K(e.value):{status:`error`,error:e.error};var Se=class e{#e;#t;constructor(e){this.#e=e}static create(t){let n;try{n=xe(t)}catch(e){return q(`initFailed`,`the palette couldn't be read: ${e.message}`)}let r=C.create(n);return r.status===`ok`?K(new e(r.value)):q(`initFailed`,r.error.message)}get paletteSize(){return this.#e.length}get image(){return this.#t&&{width:this.#t.width,height:this.#t.height}}open(e,t,n){let r=G(t,n);if(r)return q(`imageTooLarge`,r);let i=w.create(e,t,n);return i.status===`error`?{status:`error`,error:i.error}:(this.#t?.free(),this.#t=i.value,K({width:t,height:n}))}close(){this.#t?.free(),this.#t=void 0}dispose(){this.close(),this.#e.free()}analyze(e){return this.#t?Y(this.#t.analyze(e)):q(`noImage`,`no image is open`)}colorCount(e){if(!this.#t)return q(`noImage`,`no image is open`);let t=Y(this.#t.colorCount(e));return t.status===`ok`?K(t.value.colors):t}pick(e,t,n,r){return this.#t?Y(this.#t.pick(e,t,r,n,this.#e)):q(`noImage`,`no image is open`)}composite(e,t){return Y(ae(e,t))}unprintedColors(e,t,n){let r=Y(ce({colors:e,material:t,materialRanges:n}));return r.status===`ok`?K(r.value.unprinted):r}nearest(e,t=8){let n=Y(this.#e.nearest(e,t));return n.status===`ok`?K(n.value.matches):n}parseConfig(e){return Y(oe(e))}resolveSection(e,t){let n=Y(this.#e.resolveSection(e,t));return n.status===`ok`?K(n.value.picks):n}exportConfig(e,t,n,r=[]){let i=Y(se({imageName:e,material:t,unprinted:r,picks:n}));return i.status===`ok`?K(i.value.text):i}recolor(e,t,n=[]){if(!this.#t)return q(`noImage`,`no image is open`);let r=new Uint8Array(this.#t.width*this.#t.height*4),i=Y(this.#t.recolor({mappings:e,material:t,materialRanges:n},r));return i.status===`ok`?K(r):i}};Object.freeze({r:255,g:255,b:255}),Object.freeze({r:0,g:0,b:0});const Ce=(e,t)=>e.r===t.r&&e.g===t.g&&e.b===t.b,X=()=>q(`superseded`,`a newer image or change replaced this request`);var we=class{#e;#t;#n=0;#r=0;#i;constructor(e,t){this.#e=e,this.#t=t}async open(e,t){this.#n=Math.max(this.#n,t);let n=await this.#t.decode(e);if(n.status===`error`)return n;let{rgba:r,width:i,height:a,bitmap:o}=n.value;if(t!==this.#n)return o.close(),X();let s=this.#e.open(r,i,a);return s.status===`error`?(o.close(),s):(this.#r=t,this.#i=void 0,K({generation:t,width:i,height:a,bitmap:o}))}colorCount(e,t){return e===this.#r?this.#e.colorCount(t):X()}pick(e,t,n,r,i){return e===this.#r?this.#e.pick(t,n,r,i):X()}rematch(e,t){let n=[];for(let r of e){let e=this.#e.composite(r.pixel,t);if(e.status===`error`)return e;if(Ce(e.value,r.matching)){n.push({matching:e.value});continue}let i=this.#e.nearest(e.value);if(i.status===`error`)return i;n.push({matching:e.value,alternatives:i.value})}return K(n)}nearest(e,t){return this.#e.nearest(e,t)}unprintedColors(e,t,n){return this.#e.unprintedColors(e,t,n)}async recolor(e,t,n,r,i=[]){let a=this.#e.image;if(e!==this.#r||!a)return X();let o=this.#e.recolor(n,r,i);if(o.status===`error`)return o;this.#i={generation:e,revision:t,rgba:o.value};let s=await this.#t.toBitmap(o.value,a.width,a.height);return K({generation:e,revision:t,bitmap:s})}async encodePng(e,t){let n=this.#e.image,r=this.#i;if(!r||!n||r.generation!==e||r.revision!==t)return X();let i=await this.#t.encodePng(r.rgba,n.width,n.height);return this.#i===r&&this.#r===e?K(i):X()}parseConfig(e){return this.#e.parseConfig(e)}resolveSection(e,t){let n=this.#e.resolveSection(e,t);return n.status===`error`?n:K(n.value.map(e=>{let t=this.#e.nearest(e.matching),n=t.status===`ok`?t.value:[];return n.some(t=>t.index===e.ink.index)||n.push(e.ink),{...e,alternatives:n}}))}exportConfig(e,t,n,r){return this.#e.exportConfig(e,t,n,r)}};let Z;const Te=(async()=>{try{await me({module_or_path:he})}catch(e){return q(`initFailed`,`the engine couldn't start: ${String(e)}`)}let e=Se.create(ge);return e.status===`error`?e:(Z=new we(e.value,{decode:ve,toBitmap:ye,encodePng:be}),K({paletteSize:e.value.paletteSize}))})();async function Q(e){let t=await Te;return t.status===`error`?t:Z?e(Z):q(`notReady`,`the engine is not ready`)}function $(e){return e.status===`ok`?y(e,[e.value.bitmap]):e}c({ready:()=>Te,open:async(e,t)=>$(await Q(n=>n.open(e,t))),colorCount:(e,t)=>Q(n=>n.colorCount(e,t)),pick:(e,t,n,r,i)=>Q(a=>a.pick(e,t,n,r,i)),rematch:(e,t)=>Q(n=>n.rematch(e,t)),nearest:(e,t)=>Q(n=>n.nearest(e,t)),unprintedColors:(e,t,n)=>Q(r=>r.unprintedColors(e,t,n)),recolor:async(e,t,n,r,i)=>$(await Q(a=>a.recolor(e,t,n,r,i))),encodePng:(e,t)=>Q(n=>n.encodePng(e,t)),parseConfig:e=>Q(t=>t.parseConfig(e)),resolveSection:(e,t)=>Q(n=>n.resolveSection(e,t)),exportConfig:(e,t,n,r)=>Q(i=>i.exportConfig(e,t,n,r))});