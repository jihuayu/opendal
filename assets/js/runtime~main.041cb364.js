(()=>{"use strict";var e,a,c,d,f,t={},b={};function r(e){var a=b[e];if(void 0!==a)return a.exports;var c=b[e]={exports:{}};return t[e].call(c.exports,c,c.exports,r),c.exports}r.m=t,e=[],r.O=(a,c,d,f)=>{if(!c){var t=1/0;for(i=0;i<e.length;i++){c=e[i][0],d=e[i][1],f=e[i][2];for(var b=!0,o=0;o<c.length;o++)(!1&f||t>=f)&&Object.keys(r.O).every((e=>r.O[e](c[o])))?c.splice(o--,1):(b=!1,f<t&&(t=f));if(b){e.splice(i--,1);var n=d();void 0!==n&&(a=n)}}return a}f=f||0;for(var i=e.length;i>0&&e[i-1][2]>f;i--)e[i]=e[i-1];e[i]=[c,d,f]},r.n=e=>{var a=e&&e.__esModule?()=>e.default:()=>e;return r.d(a,{a:a}),a},c=Object.getPrototypeOf?e=>Object.getPrototypeOf(e):e=>e.__proto__,r.t=function(e,d){if(1&d&&(e=this(e)),8&d)return e;if("object"==typeof e&&e){if(4&d&&e.__esModule)return e;if(16&d&&"function"==typeof e.then)return e}var f=Object.create(null);r.r(f);var t={};a=a||[null,c({}),c([]),c(c)];for(var b=2&d&&e;"object"==typeof b&&!~a.indexOf(b);b=c(b))Object.getOwnPropertyNames(b).forEach((a=>t[a]=()=>e[a]));return t.default=()=>e,r.d(f,t),f},r.d=(e,a)=>{for(var c in a)r.o(a,c)&&!r.o(e,c)&&Object.defineProperty(e,c,{enumerable:!0,get:a[c]})},r.f={},r.e=e=>Promise.all(Object.keys(r.f).reduce(((a,c)=>(r.f[c](e,a),a)),[])),r.u=e=>"assets/js/"+({53:"935f2afb",217:"b56f9725",380:"ab10c306",455:"07df3158",533:"b2b675dd",716:"a0405932",990:"592d1076",1011:"cc62039a",1163:"434bbc94",1372:"1db64337",1477:"b2f554cd",1513:"9897d641",1652:"8cc9519e",1713:"a7023ddc",1846:"054cd2fb",2379:"d5c31a96",2505:"14a24c8e",2535:"814f3328",2652:"8bf4b5be",2804:"2cbf1097",3057:"136e1c50",3085:"1f391b9e",3089:"a6aa9e1f",3358:"f3200a52",3608:"9e4087bc",3661:"3806ecb0",3845:"e892cec6",3880:"7caca07b",3922:"6eb7fe18",4013:"01a85c17",4157:"283e63f8",4195:"c4f5d8e4",4277:"939f74f6",4477:"1957547a",4536:"22d8fa67",4538:"f1ca5a39",4719:"8f4159f2",4790:"d3c0d8d5",4832:"ece86388",5214:"bb5b2057",5555:"37896f57",5606:"beae818e",5822:"be97ea89",6089:"62a401e9",6103:"ccc49370",6152:"29b208a4",6299:"8b18d929",6427:"5a65e608",6578:"b9ae8b37",6611:"209227ae",6915:"093f5f04",6948:"e19a6781",6983:"ae4554eb",7289:"c6ab695b",7309:"4fb2b91a",7757:"57a16c1d",7794:"02514dc9",7918:"17896441",8610:"6875c492",9319:"4b02d014",9514:"1be78505",9817:"14eb3368",9826:"ddccef4c"}[e]||e)+"."+{53:"974f4713",217:"2c0d6bce",380:"d5666577",455:"0f01c23c",533:"856b2c0d",716:"4a7f1e1f",990:"899528ef",1011:"ae14dae5",1163:"f743262d",1372:"f9ea1ba5",1477:"ba45221c",1506:"7a372cde",1513:"f2f7d7d4",1652:"ae62a53e",1713:"7a4d520b",1846:"1cf06ffc",2379:"ab5c2f4d",2505:"5b9af604",2529:"ea72b266",2535:"f44033d6",2652:"844f323c",2804:"37131de4",3057:"4836baab",3085:"fc747d21",3089:"66019f66",3358:"117ebfb8",3608:"822525bd",3661:"7d81778c",3845:"0cb21f3d",3880:"31b6335a",3922:"c3db6f05",4013:"4316cd6e",4157:"a74b6771",4195:"acdec80a",4277:"4a5db21a",4477:"73f5a6aa",4536:"543baac2",4538:"4e61ecc7",4719:"c19cd3cc",4790:"65e2fbec",4832:"c7f148e3",4972:"f7d54065",5214:"1f0378d8",5555:"fc5a447b",5606:"d058c3d4",5822:"1a0bbe2e",6089:"07dccd13",6103:"ba4b09b5",6152:"736060be",6299:"331315e2",6427:"a78a6469",6578:"7d1b1266",6611:"fde3d3f7",6915:"653001c0",6948:"8576bf3f",6983:"64b904d0",7289:"0d66ec11",7309:"4e640a3e",7757:"60688a15",7794:"d1218e3f",7918:"fa99442c",8610:"a3ea2b2f",9319:"a0a3ee64",9514:"d888abe5",9817:"5d4ade20",9826:"a1b6388f"}[e]+".js",r.miniCssF=e=>{},r.g=function(){if("object"==typeof globalThis)return globalThis;try{return this||new Function("return this")()}catch(e){if("object"==typeof window)return window}}(),r.o=(e,a)=>Object.prototype.hasOwnProperty.call(e,a),d={},f="opendal-website:",r.l=(e,a,c,t)=>{if(d[e])d[e].push(a);else{var b,o;if(void 0!==c)for(var n=document.getElementsByTagName("script"),i=0;i<n.length;i++){var u=n[i];if(u.getAttribute("src")==e||u.getAttribute("data-webpack")==f+c){b=u;break}}b||(o=!0,(b=document.createElement("script")).charset="utf-8",b.timeout=120,r.nc&&b.setAttribute("nonce",r.nc),b.setAttribute("data-webpack",f+c),b.src=e),d[e]=[a];var l=(a,c)=>{b.onerror=b.onload=null,clearTimeout(s);var f=d[e];if(delete d[e],b.parentNode&&b.parentNode.removeChild(b),f&&f.forEach((e=>e(c))),a)return a(c)},s=setTimeout(l.bind(null,void 0,{type:"timeout",target:b}),12e4);b.onerror=l.bind(null,b.onerror),b.onload=l.bind(null,b.onload),o&&document.head.appendChild(b)}},r.r=e=>{"undefined"!=typeof Symbol&&Symbol.toStringTag&&Object.defineProperty(e,Symbol.toStringTag,{value:"Module"}),Object.defineProperty(e,"__esModule",{value:!0})},r.p="/",r.gca=function(e){return e={17896441:"7918","935f2afb":"53",b56f9725:"217",ab10c306:"380","07df3158":"455",b2b675dd:"533",a0405932:"716","592d1076":"990",cc62039a:"1011","434bbc94":"1163","1db64337":"1372",b2f554cd:"1477","9897d641":"1513","8cc9519e":"1652",a7023ddc:"1713","054cd2fb":"1846",d5c31a96:"2379","14a24c8e":"2505","814f3328":"2535","8bf4b5be":"2652","2cbf1097":"2804","136e1c50":"3057","1f391b9e":"3085",a6aa9e1f:"3089",f3200a52:"3358","9e4087bc":"3608","3806ecb0":"3661",e892cec6:"3845","7caca07b":"3880","6eb7fe18":"3922","01a85c17":"4013","283e63f8":"4157",c4f5d8e4:"4195","939f74f6":"4277","1957547a":"4477","22d8fa67":"4536",f1ca5a39:"4538","8f4159f2":"4719",d3c0d8d5:"4790",ece86388:"4832",bb5b2057:"5214","37896f57":"5555",beae818e:"5606",be97ea89:"5822","62a401e9":"6089",ccc49370:"6103","29b208a4":"6152","8b18d929":"6299","5a65e608":"6427",b9ae8b37:"6578","209227ae":"6611","093f5f04":"6915",e19a6781:"6948",ae4554eb:"6983",c6ab695b:"7289","4fb2b91a":"7309","57a16c1d":"7757","02514dc9":"7794","6875c492":"8610","4b02d014":"9319","1be78505":"9514","14eb3368":"9817",ddccef4c:"9826"}[e]||e,r.p+r.u(e)},(()=>{var e={1303:0,532:0};r.f.j=(a,c)=>{var d=r.o(e,a)?e[a]:void 0;if(0!==d)if(d)c.push(d[2]);else if(/^(1303|532)$/.test(a))e[a]=0;else{var f=new Promise(((c,f)=>d=e[a]=[c,f]));c.push(d[2]=f);var t=r.p+r.u(a),b=new Error;r.l(t,(c=>{if(r.o(e,a)&&(0!==(d=e[a])&&(e[a]=void 0),d)){var f=c&&("load"===c.type?"missing":c.type),t=c&&c.target&&c.target.src;b.message="Loading chunk "+a+" failed.\n("+f+": "+t+")",b.name="ChunkLoadError",b.type=f,b.request=t,d[1](b)}}),"chunk-"+a,a)}},r.O.j=a=>0===e[a];var a=(a,c)=>{var d,f,t=c[0],b=c[1],o=c[2],n=0;if(t.some((a=>0!==e[a]))){for(d in b)r.o(b,d)&&(r.m[d]=b[d]);if(o)var i=o(r)}for(a&&a(c);n<t.length;n++)f=t[n],r.o(e,f)&&e[f]&&e[f][0](),e[f]=0;return r.O(i)},c=self.webpackChunkopendal_website=self.webpackChunkopendal_website||[];c.forEach(a.bind(null,0)),c.push=a.bind(null,c.push.bind(c))})()})();