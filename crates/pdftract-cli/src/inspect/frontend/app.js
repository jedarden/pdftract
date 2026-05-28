// pdftract inspector - Phase 7.9.3 frontend bundle
// Single-page vanilla web app, <80KB gzipped, no framework, no CDN

const STORAGE_PREFIX='pdftract-inspector-';
const LAYERS=['spans','blocks','columns','reading-order','confidence-heatmap','ocr','mcid','anchors'];
const LAYER_KEYS=['spans','blocks','columns','reading_order','confidence_heatmap','ocr','mcid','anchors'];

let currentPage=0;
let totalPages=0;
let pageData=null;

function init(){loadLayerState();setupKeyboard();setupToggles();setupSearch();setupNav();loadFragment()}
async function loadDocument(){const res=await fetch('/api/document');if(!res.ok)throw new Error('Failed to load document');const data=await res.json();totalPages=data.pages?.length||0;renderThumbnails();loadFragment()}
async function loadPage(index){const res=await fetch(`/api/page/${index}`);if(!res.ok)throw new Error('Failed to load page');pageData=await res.json();currentPage=index;renderPage();renderJson();updateActiveThumbnail();updateFragment();updateNavState()}
async function loadThumbnails(){const container=document.getElementById('thumbnails');container.innerHTML='';for(let i=0;i<totalPages;i++){const thumb=document.createElement('div');thumb.className='thumbnail';thumb.dataset.index=i;const img=document.createElement('img');img.className='thumbnail-img';img.src=`/api/page/${i}/thumbnail`;img.alt=`Page ${i+1}`;img.loading='lazy';const num=document.createElement('div');num.className='thumbnail-number';num.textContent=`${i+1}`;thumb.appendChild(img);thumb.appendChild(num);thumb.addEventListener('click',()=>loadPage(i));container.appendChild(thumb)}}
function renderThumbnails(){loadThumbnails()}
async function renderPage(){const container=document.getElementById('canvas-container');container.innerHTML='';const res=await fetch(`/api/page/${currentPage}/svg`);if(!res.ok)throw new Error('Failed to load SVG');const svg=await res.text();const wrapper=document.createElement('div');wrapper.id='page-svg';wrapper.innerHTML=svg;setupTooltips(wrapper);container.appendChild(wrapper)}
function renderJson(){const tree=document.getElementById('json-tree');tree.textContent=JSON.stringify(pageData,null,2)}
function loadLayerState(){const stored=localStorage.getItem(STORAGE_PREFIX+'layers');const active=stored?stored.split(','):[];applyLayers(active)}
function saveLayerState(active){localStorage.setItem(STORAGE_PREFIX+'layers',active.join(','))}
function applyLayers(active){document.documentElement.dataset.layers=active.join(',');document.querySelectorAll('.layer-toggle').forEach(btn=>{const layer=btn.dataset.layer;btn.classList.toggle('active',active.includes(layer))})}
function toggleLayer(layer){const current=document.documentElement.dataset.layers.split(',').filter(Boolean);const idx=current.indexOf(layer);if(idx>=0)current.splice(idx,1);else current.push(layer);saveLayerState(current);applyLayers(current)}
function setupToggles(){document.querySelectorAll('.layer-toggle').forEach(btn=>{btn.addEventListener('click',()=>toggleLayer(btn.dataset.layer))})}
function setupKeyboard(){document.addEventListener('keydown',e=>{if(e.target.tagName==='INPUT')return;if(e.key==='ArrowLeft')e.preventDefault(),navigatePage(-1);else if(e.key==='ArrowRight')e.preventDefault(),navigatePage(1);else if(e.key==='/')e.preventDefault(),document.getElementById('search-input').focus();else if(e.key>='1'&&e.key<='8'){const idx=parseInt(e.key)-1;const layer=LAYERS[idx];if(layer)toggleLayer(layer)}})}
function setupSearch(){const input=document.getElementById('search-input');let timeout;input.addEventListener('input',()=>{clearTimeout(timeout);timeout=setTimeout(performSearch,300)})}
async function performSearch(){const query=document.getElementById('search-input').value.trim();if(!query)return;const res=await fetch(`/api/search?q=${encodeURIComponent(query)}`);if(!res.ok)return;const matches=await res.json();if(matches.length>0){const match=matches[0];if(match.page_index!==currentPage)loadPage(match.page_index)}}
function setupNav(){document.getElementById('btn-prev').addEventListener('click',()=>navigatePage(-1));document.getElementById('btn-next').addEventListener('click',()=>navigatePage(1))}
function navigatePage(delta){const newPage=currentPage+delta;if(newPage>=0&&newPage<totalPages)loadPage(newPage)}
function updateNavState(){document.getElementById('btn-prev').disabled=currentPage<=0;document.getElementById('btn-next').disabled=currentPage>=totalPages-1}
function updateActiveThumbnail(){document.querySelectorAll('.thumbnail').forEach(t=>t.classList.toggle('active',parseInt(t.dataset.index)===currentPage))}
function updateFragment(){history.replaceState(null,'',`#page=${currentPage}`)}
function loadFragment(){const match=/#page=(\d+)/.exec(location.hash);if(match){const page=parseInt(match[1]);if(page>=0)page<totalPages?loadPage(page):loadDocument().then(()=>page<totalPages&&loadPage(page))}else loadDocument()}
function setupTooltips(svg){const tooltip=document.getElementById('tooltip');svg.addEventListener('mouseover',e=>{const target=e.target.closest('[data-tooltip]');if(!target)return;tooltip.hidden=false;tooltip.textContent=target.dataset.tooltip;tooltip.style.left=e.pageX+10+'px';tooltip.style.top=e.pageY+10+'px'});svg.addEventListener('mouseout',e=>{if(e.target.closest('[data-tooltip]'))tooltip.hidden=true});svg.addEventListener('mousemove',e=>{if(!tooltip.hidden){tooltip.style.left=e.pageX+10+'px';tooltip.style.top=e.pageY+10+'px'}})}
document.addEventListener('DOMContentLoaded',init);