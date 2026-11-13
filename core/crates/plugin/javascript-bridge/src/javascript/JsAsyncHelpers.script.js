/*! @oxc-project/runtime 0.146.0: async helpers
MIT License

Copyright (c) 2024-present VoidZero Inc. & Contributors
Copyright (c) 2023 Boshen

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
*/
var __operitAsyncHelpers=(()=>{var f=(r,e)=>()=>(e||r((e={exports:{}}).exports,e),e.exports);var G=f((B,y)=>{function k(r,e,t,o,n,u,i){try{var a=r[u](i),s=a.value}catch(l){return void t(l)}a.done?e(s):Promise.resolve(s).then(o,n)}function A(r){return function(){var e=this,t=arguments;return new Promise(function(o,n){var u=r.apply(e,t);function i(s){k(u,o,n,i,a,"next",s)}function a(s){k(u,o,n,i,a,"throw",s)}i(void 0)})}}y.exports=A,y.exports.__esModule=!0,y.exports.default=y.exports});var _=f((H,v)=>{function j(r,e){this.v=r,this.k=e}v.exports=j,v.exports.__esModule=!0,v.exports.default=v.exports});var g=f((J,h)=>{var M=_();function O(r){return function(){return new d(r.apply(this,arguments))}}function d(r){var e,t;function o(u,i){try{var a=r[u](i),s=a.value,l=s instanceof M;Promise.resolve(l?s.v:s).then(function(c){if(l){var S=u==="return"?"return":"next";if(!s.k||c.done)return o(S,c);c=r[S](c).value}n(a.done?"return":"normal",c)},function(c){o("throw",c)})}catch(c){n("throw",c)}}function n(u,i){switch(u){case"return":e.resolve({value:i,done:!0});break;case"throw":e.reject(i);break;default:e.resolve({value:i,done:!1})}(e=e.next)?o(e.key,e.arg):t=null}this._invoke=function(u,i){return new Promise(function(a,s){var l={key:u,arg:i,resolve:a,reject:s,next:null};t?t=t.next=l:(e=t=l,o(u,i))})},typeof r.return!="function"&&(this.return=void 0)}d.prototype[typeof Symbol=="function"&&Symbol.asyncIterator||"@@asyncIterator"]=function(){return this},d.prototype.next=function(r){return this._invoke("next",r)},d.prototype.throw=function(r){return this._invoke("throw",r)},d.prototype.return=function(r){return this._invoke("return",r)};h.exports=O,h.exports.__esModule=!0,h.exports.default=h.exports});var I=f((K,x)=>{var T=_();function Y(r){return new T(r,0)}x.exports=Y,x.exports.__esModule=!0,x.exports.default=x.exports});var P=f((L,w)=>{var F=_();function D(r){var e={},t=!1;function o(n,u){return t=!0,u=new Promise(function(i){i(r[n](u))}),{done:!1,value:new F(u,1)}}return e[typeof Symbol!="undefined"&&Symbol.iterator||"@@iterator"]=function(){return this},e.next=function(n){return t?(t=!1,n):o("next",n)},typeof r.throw=="function"&&(e.throw=function(n){if(t)throw t=!1,n;return o("throw",n)}),typeof r.return=="function"&&(e.return=function(n){return t?(t=!1,n):o("return",n)}),e}w.exports=D,w.exports.__esModule=!0,w.exports.default=w.exports});var q=f((N,m)=>{function E(r){var e,t,o,n=2;for(typeof Symbol!="undefined"&&(t=Symbol.asyncIterator,o=Symbol.iterator);n--;){if(t&&(e=r[t])!=null)return e.call(r);if(o&&(e=r[o])!=null)return new b(e.call(r));t="@@asyncIterator",o="@@iterator"}throw new TypeError("Object is not async iterable")}function b(r){function e(t){if(Object(t)!==t)return Promise.reject(new TypeError(t+" is not an object."));var o=t.done;return Promise.resolve(t.value).then(function(n){return{value:n,done:o}})}return b=function(o){this.s=o,this.n=o.next},b.prototype={s:null,n:null,next:function(){return e(this.n.apply(this.s,arguments))},return:function(o){var n=this.s.return;return n===void 0?Promise.resolve({value:o,done:!0}):e(n.apply(this.s,arguments))},throw:function(o){var n=this.s.return;return n===void 0?Promise.reject(o):e(n.apply(this.s,arguments))}},new b(r)}m.exports=E,m.exports.__esModule=!0,m.exports.default=m.exports});var C=f(p=>{p.asyncToGenerator=G();p.wrapAsyncGenerator=g();p.awaitAsyncGenerator=I();p.asyncGeneratorDelegate=P();p.asyncIterator=q()});return C();})();
/** Preserves serializable async source across compiler-generated wrapper closures. */
__operitAsyncHelpers.remember = function(fn, source) {
    Object.defineProperty(fn, 'toString', {
        /** Returns the source that the destination runtime compiles through its normal entry. */
        value: function() { return source; },
        configurable: true
    });
    return fn;
};
