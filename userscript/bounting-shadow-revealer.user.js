// ==UserScript==
// @name         Bounting Shadow Revealer（TurboWarp 影子积木显现）
// @namespace    bounting
// @version      0.1.0
// @description  在 TurboWarp 编辑器中以半透明方式显现所有影子/输入积木，并可将它们拖出变成实体积木。保持原版编辑器风格，不改任何配色与布局。
// @author       myAtomCode (Bounting)
// @match        https://turbowarp.org/*
// @match        https://lab.turbowarp.org/*
// @run-at       document-idle
// @grant        none
// @license      MIT
// ==/UserScript==

/*
 * 用法：
 *   - 点击编辑器底部中间的「影子显现」胶囊按钮，或按 Alt+S 开关显现模式。
 *   - 显现模式下：所有影子积木（输入框里的默认占位积木，如「移动 (10) 步」的 10）
 *     以 45% 透明度显示，悬停时加深。
 *   - 拖动显现出的影子积木（移动超过 4px）＝ 把它拽出来变成实体积木，
 *     原输入槽会自动重生一个新的默认影子，和手动拖入积木的行为完全一致。
 *   - 单击不拖动＝维持原版行为：数字/文本框正常输入，下拉框正常展开菜单。
 *   - 按住 Alt 点击影子积木＝立即拽出（跳过移动阈值）。
 *   - 开关状态记忆在 localStorage，刷新后保持。
 */

(function () {
  'use strict';

  var STORE_KEY = 'bounting.shadowReveal';
  var enabled = false;
  try { enabled = localStorage.getItem(STORE_KEY) === '1'; } catch (e) { /* 隐私模式等 */ }

  var pending = null;          // { block, x, y, alt } 待判定拖拽意图的影子
  var internal = false;        // 正在派发合成事件，跳过自家拦截器
  var applyTimer = null;

  /* ---------- 样式：全部走 Scratch 原生视觉语言 ---------- */
  var style = document.createElement('style');
  style.textContent = [
    // 影子显现：只调透明度，不动形状/颜色，保持原版风格
    '.bounting-shadow-reveal { opacity: 0.45 !important; transition: opacity .15s ease; }',
    '.bounting-shadow-reveal:hover { opacity: 0.75 !important; }',
    // 开关按钮：仿 Scratch 舞台控制区的白色胶囊
    '#bounting-shadow-btn {',
    '  position: fixed; bottom: 14px; left: 50%; transform: translateX(-50%);',
    '  z-index: 99999; display: flex; align-items: center; gap: 6px;',
    '  padding: 6px 14px; border-radius: 16px; cursor: pointer; user-select: none;',
    '  background: #ffffff; color: #575e75; border: 1px solid rgba(0,0,0,.15);',
    '  box-shadow: 0 2px 5px rgba(0,0,0,.15);',
    '  font: 600 12px/1 "Helvetica Neue", Helvetica, "PingFang SC", "Microsoft YaHei", sans-serif;',
    '  transition: background .15s ease, color .15s ease;',
    '}',
    '#bounting-shadow-btn .bounting-dot {',
    '  width: 8px; height: 8px; border-radius: 50%; background: #b9bcc6;',
    '  transition: background .15s ease;',
    '}',
    '#bounting-shadow-btn.on { background: #4c97ff; color: #ffffff; border-color: #4280c7; }',
    '#bounting-shadow-btn.on .bounting-dot { background: #ffffff; }'
  ].join('\n');
  document.documentElement.appendChild(style);

  /* ---------- 等待编辑器就绪（Blockly 全局在打开编辑器后才存在） ---------- */
  var waitTimer = setInterval(function () {
    var B = window.Blockly;
    if (B && B.getMainWorkspace) {
      var ws = null;
      try { ws = B.getMainWorkspace(); } catch (e) { ws = null; }
      if (ws && ws.getCanvas && ws.getCanvas()) {
        clearInterval(waitTimer);
        init(B, ws);
      }
    }
  }, 300);

  function init(Blockly, ws) {
    buildButton();
    bindDragInterceptor(Blockly, ws);

    // 工作区变化（切角色/加载工程/撤销/拖入积木）后重新施加显现样式
    if (ws.addChangeListener) {
      ws.addChangeListener(function () {
        if (!enabled) return;
        clearTimeout(applyTimer);
        applyTimer = setTimeout(function () { applyReveal(Blockly); }, 80);
      });
    }
    if (enabled) applyReveal(Blockly);
    refreshButton();
  }

  /* ---------- 收集工作区里的所有块（优先 blockDB_，包含影子） ---------- */
  function collectBlocks(ws) {
    try {
      if (ws.blockDB_) {
        var out = [];
        for (var k in ws.blockDB_) {
          var b = ws.blockDB_[k];
          if (b && b.svgGroup_) out.push(b);
        }
        return out;
      }
    } catch (e) { /* 走兜底 */ }
    return ws.getAllBlocks ? ws.getAllBlocks(false) : [];
  }

  function applyReveal(Blockly) {
    var ws = Blockly.getMainWorkspace();
    if (!ws) return;
    var blocks = collectBlocks(ws);
    for (var i = 0; i < blocks.length; i++) {
      var b = blocks[i];
      var g = b.svgGroup_;
      if (!g || !g.classList) continue;
      if (enabled && b.isShadow && b.isShadow()) g.classList.add('bounting-shadow-reveal');
      else g.classList.remove('bounting-shadow-reveal');
    }
  }

  /* ---------- 拖拽拦截：把影子「拽出」为实体积木 ---------- */
  function bindDragInterceptor(Blockly, ws) {
    var svg = ws.svgGroup_ || ws.getCanvas() || document.querySelector('.blocklySvg');
    if (!svg) return;

    svg.addEventListener('mousedown', function (e) {
      if (internal || e.button !== 0 || !enabled) return;
      var block = findShadowBlock(Blockly, e.target);
      if (!block) return;
      pending = { block: block, x: e.clientX, y: e.clientY, alt: e.altKey };
      if (e.altKey) {
        // Alt+点击：立即拽出，不需要移动阈值
        pullOut(Blockly, block, e.clientX, e.clientY);
        pending = null;
      }
      // 不阻断默认行为：普通点击仍然是原版的字段编辑/下拉菜单
    }, true);

    svg.addEventListener('mousemove', function (e) {
      if (internal || !pending || !(e.buttons & 1)) { if (!(e.buttons & 1)) pending = null; return; }
      var dx = e.clientX - pending.x, dy = e.clientY - pending.y;
      if (dx * dx + dy * dy < 16) return; // 4px 阈值：小于这个视为点击
      pullOut(Blockly, pending.block, e.clientX, e.clientY);
      pending = null;
    }, true);

    svg.addEventListener('mouseup', function () { pending = null; }, true);
  }

  function findShadowBlock(Blockly, node) {
    var ws = Blockly.getMainWorkspace();
    if (!ws) return null;
    var blocks = collectBlocks(ws);
    for (var i = 0; i < blocks.length; i++) {
      var b = blocks[i];
      if (b.isShadow && b.isShadow() && b.svgGroup_ && b.svgGroup_.contains(node)) return b;
    }
    return null;
  }

  /** 核心：影子 → 实体积木，再合成鼠标事件让 Blockly 接管拖拽 */
  function pullOut(Blockly, block, x, y) {
    if (!block || !block.isShadow || !block.isShadow()) return;
    var g = block.svgGroup_;
    if (g) g.classList.remove('bounting-shadow-reveal');

    // 1) 结束当前手势（原 mousedown 可能让 Blockly 绑定了父块拖拽/字段编辑）
    fire('mouseup', g, x, y);

    // 2) 影子转实体：去掉影子标记并完整重渲染（补齐缺口/连接口）
    block.setShadow(false);
    try { block.initSvg(); } catch (e) { /* 已初始化则忽略 */ }
    try { block.render(false); } catch (e) { /* 渲染失败不致命 */ }

    // 3) 合成按下+移动，让 Blockly 以实体积木的身份开始拖拽；
    //    拖离输入槽时原槽位会自动重生新的默认影子（scratch-blocks 原生行为）
    internal = true;
    try {
      var md = fire('mousedown', g, x, y);
      fire('mousemove', g, x, y);
      // mousedown 冒泡到 svg 根后 Gesture 已建立，恢复拦截
      setTimeout(function () { internal = false; }, 0);
      void md;
    } catch (e) {
      internal = false;
    }
  }

  function fire(type, target, x, y) {
    var ev = new MouseEvent(type, {
      bubbles: true, cancelable: true, view: window,
      clientX: x, clientY: y, button: 0, buttons: type === 'mouseup' ? 0 : 1
    });
    target.dispatchEvent(ev);
    return ev;
  }

  /* ---------- 原版风格开关按钮 ---------- */
  function buildButton() {
    if (document.getElementById('bounting-shadow-btn')) return;
    var btn = document.createElement('div');
    btn.id = 'bounting-shadow-btn';
    btn.title = '影子积木显现（Alt+S）\n显现后拖动影子积木可拽出为实体积木';
    btn.innerHTML = '<span class="bounting-dot"></span><span class="bounting-label">影子显现</span>';
    btn.addEventListener('click', function () { toggle(); });
    document.body.appendChild(btn);
  }

  function refreshButton() {
    var btn = document.getElementById('bounting-shadow-btn');
    if (btn) btn.classList.toggle('on', enabled);
    var label = btn && btn.querySelector('.bounting-label');
    if (label) label.textContent = enabled ? '影子显现：开' : '影子显现';
  }

  function toggle() {
    enabled = !enabled;
    try { localStorage.setItem(STORE_KEY, enabled ? '1' : '0'); } catch (e) { /* 忽略 */ }
    var B = window.Blockly;
    if (B) applyReveal(B);
    refreshButton();
  }

  window.addEventListener('keydown', function (e) {
    if (e.altKey && (e.key === 's' || e.key === 'S')) {
      e.preventDefault();
      toggle();
    }
  });
})();
