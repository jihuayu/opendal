(()=>{"use strict";var e,t,r,a,o,n={},f={};function c(e){var t=f[e];if(void 0!==t)return t.exports;var r=f[e]={exports:{}};return n[e].call(r.exports,r,r.exports,c),r.exports}c.m=n,e=[],c.O=(t,r,a,o)=>{if(!r){var n=1/0;for(i=0;i<e.length;i++){r=e[i][0],a=e[i][1],o=e[i][2];for(var f=!0,b=0;b<r.length;b++)(!1&o||n>=o)&&Object.keys(c.O).every((e=>c.O[e](r[b])))?r.splice(b--,1):(f=!1,o<n&&(n=o));if(f){e.splice(i--,1);var d=a();void 0!==d&&(t=d)}}return t}o=o||0;for(var i=e.length;i>0&&e[i-1][2]>o;i--)e[i]=e[i-1];e[i]=[r,a,o]},c.n=e=>{var t=e&&e.__esModule?()=>e.default:()=>e;return c.d(t,{a:t}),t},r=Object.getPrototypeOf?e=>Object.getPrototypeOf(e):e=>e.__proto__,c.t=function(e,a){if(1&a&&(e=this(e)),8&a)return e;if("object"==typeof e&&e){if(4&a&&e.__esModule)return e;if(16&a&&"function"==typeof e.then)return e}var o=Object.create(null);c.r(o);var n={};t=t||[null,r({}),r([]),r(r)];for(var f=2&a&&e;"object"==typeof f&&!~t.indexOf(f);f=r(f))Object.getOwnPropertyNames(f).forEach((t=>n[t]=()=>e[t]));return n.default=()=>e,c.d(o,n),o},c.d=(e,t)=>{for(var r in t)c.o(t,r)&&!c.o(e,r)&&Object.defineProperty(e,r,{enumerable:!0,get:t[r]})},c.f={},c.e=e=>Promise.all(Object.keys(c.f).reduce(((t,r)=>(c.f[r](e,t),t)),[])),c.u=e=>"assets/js/"+({13:"01a85c17",53:"935f2afb",89:"62a401e9",103:"ccc49370",157:"283e63f8",195:"c4f5d8e4",301:"b2f554cd",309:"4fb2b91a",358:"f3200a52",372:"1db64337",455:"07df3158",477:"1957547a",514:"1be78505",533:"b2b675dd",535:"814f3328",608:"9e4087bc",610:"6875c492",661:"3806ecb0",713:"a7023ddc",716:"a0405932",719:"8f4159f2",757:"57a16c1d",794:"02514dc9",817:"14eb3368",832:"ece86388",886:"a6aa9e1f",918:"17896441",948:"e19a6781",983:"ae4554eb"}[e]||e)+"."+{13:"3397570e",53:"b07ccbce",89:"ae5f7b80",103:"1536bedc",157:"c5889d28",195:"6fc9a0b5",301:"01304f6e",309:"c4e0977c",358:"3a8c6075",372:"81cf9874",455:"5ec2c321",477:"ff0354af",506:"8dd92e91",514:"bf9ed0cc",529:"2a6fecbd",533:"035f3452",535:"03be8526",608:"5d00d7d2",610:"5f8b7a56",661:"d2c86b76",713:"edd4bb6a",716:"66005b2f",719:"ee826652",757:"e7be18c6",794:"a7883f1d",817:"cfb577e6",832:"f1337e80",886:"65b2b652",918:"091b1783",948:"39489a1f",972:"5fafb8e0",983:"46bc91f5"}[e]+".js",c.miniCssF=e=>{},c.g=function(){if("object"==typeof globalThis)return globalThis;try{return this||new Function("return this")()}catch(e){if("object"==typeof window)return window}}(),c.o=(e,t)=>Object.prototype.hasOwnProperty.call(e,t),a={},o="opendal-website:",c.l=(e,t,r,n)=>{if(a[e])a[e].push(t);else{var f,b;if(void 0!==r)for(var d=document.getElementsByTagName("script"),i=0;i<d.length;i++){var u=d[i];if(u.getAttribute("src")==e||u.getAttribute("data-webpack")==o+r){f=u;break}}f||(b=!0,(f=document.createElement("script")).charset="utf-8",f.timeout=120,c.nc&&f.setAttribute("nonce",c.nc),f.setAttribute("data-webpack",o+r),f.src=e),a[e]=[t];var l=(t,r)=>{f.onerror=f.onload=null,clearTimeout(s);var o=a[e];if(delete a[e],f.parentNode&&f.parentNode.removeChild(f),o&&o.forEach((e=>e(r))),t)return t(r)},s=setTimeout(l.bind(null,void 0,{type:"timeout",target:f}),12e4);f.onerror=l.bind(null,f.onerror),f.onload=l.bind(null,f.onload),b&&document.head.appendChild(f)}},c.r=e=>{"undefined"!=typeof Symbol&&Symbol.toStringTag&&Object.defineProperty(e,Symbol.toStringTag,{value:"Module"}),Object.defineProperty(e,"__esModule",{value:!0})},c.p="/",c.gca=function(e){return e={17896441:"918","01a85c17":"13","935f2afb":"53","62a401e9":"89",ccc49370:"103","283e63f8":"157",c4f5d8e4:"195",b2f554cd:"301","4fb2b91a":"309",f3200a52:"358","1db64337":"372","07df3158":"455","1957547a":"477","1be78505":"514",b2b675dd:"533","814f3328":"535","9e4087bc":"608","6875c492":"610","3806ecb0":"661",a7023ddc:"713",a0405932:"716","8f4159f2":"719","57a16c1d":"757","02514dc9":"794","14eb3368":"817",ece86388:"832",a6aa9e1f:"886",e19a6781:"948",ae4554eb:"983"}[e]||e,c.p+c.u(e)},(()=>{var e={303:0,532:0};c.f.j=(t,r)=>{var a=c.o(e,t)?e[t]:void 0;if(0!==a)if(a)r.push(a[2]);else if(/^(303|532)$/.test(t))e[t]=0;else{var o=new Promise(((r,o)=>a=e[t]=[r,o]));r.push(a[2]=o);var n=c.p+c.u(t),f=new Error;c.l(n,(r=>{if(c.o(e,t)&&(0!==(a=e[t])&&(e[t]=void 0),a)){var o=r&&("load"===r.type?"missing":r.type),n=r&&r.target&&r.target.src;f.message="Loading chunk "+t+" failed.\n("+o+": "+n+")",f.name="ChunkLoadError",f.type=o,f.request=n,a[1](f)}}),"chunk-"+t,t)}},c.O.j=t=>0===e[t];var t=(t,r)=>{var a,o,n=r[0],f=r[1],b=r[2],d=0;if(n.some((t=>0!==e[t]))){for(a in f)c.o(f,a)&&(c.m[a]=f[a]);if(b)var i=b(c)}for(t&&t(r);d<n.length;d++)o=n[d],c.o(e,o)&&e[o]&&e[o][0](),e[o]=0;return c.O(i)},r=self.webpackChunkopendal_website=self.webpackChunkopendal_website||[];r.forEach(t.bind(null,0)),r.push=t.bind(null,r.push.bind(r))})()})();