const IS_DEV = false;
const equalFn = (a, b) => a === b;
const $TRACK = /* @__PURE__ */ Symbol("solid-track");
const signalOptions = {
  equals: equalFn
};
let runEffects = runQueue;
const STALE = 1;
const PENDING = 2;
const UNOWNED = {
  owned: null,
  cleanups: null,
  context: null,
  owner: null
};
var Owner = null;
let Transition = null;
let ExternalSourceConfig = null;
let Listener = null;
let Updates = null;
let Effects = null;
let ExecCount = 0;
function createRoot(fn, detachedOwner) {
  const listener = Listener, owner = Owner, unowned = fn.length === 0, current = detachedOwner === void 0 ? owner : detachedOwner, root = unowned ? UNOWNED : {
    owned: null,
    cleanups: null,
    context: current ? current.context : null,
    owner: current
  }, updateFn = unowned ? fn : () => fn(() => untrack(() => cleanNode(root)));
  Owner = root;
  Listener = null;
  try {
    return runUpdates(updateFn, true);
  } finally {
    Listener = listener;
    Owner = owner;
  }
}
function createSignal(value, options) {
  options = options ? Object.assign({}, signalOptions, options) : signalOptions;
  const s = {
    value,
    observers: null,
    observerSlots: null,
    comparator: options.equals || void 0
  };
  const setter = (value2) => {
    if (typeof value2 === "function") {
      value2 = value2(s.value);
    }
    return writeSignal(s, value2);
  };
  return [readSignal.bind(s), setter];
}
function createRenderEffect(fn, value, options) {
  const c = createComputation(fn, value, false, STALE);
  updateComputation(c);
}
function createEffect(fn, value, options) {
  runEffects = runUserEffects;
  const c = createComputation(fn, value, false, STALE);
  c.user = true;
  Effects ? Effects.push(c) : updateComputation(c);
}
function createMemo(fn, value, options) {
  options = options ? Object.assign({}, signalOptions, options) : signalOptions;
  const c = createComputation(fn, value, true, 0);
  c.observers = null;
  c.observerSlots = null;
  c.comparator = options.equals || void 0;
  updateComputation(c);
  return readSignal.bind(c);
}
function untrack(fn) {
  if (Listener === null) return fn();
  const listener = Listener;
  Listener = null;
  try {
    if (ExternalSourceConfig) ;
    return fn();
  } finally {
    Listener = listener;
  }
}
function onCleanup(fn) {
  if (Owner === null) ;
  else if (Owner.cleanups === null) Owner.cleanups = [fn];
  else Owner.cleanups.push(fn);
  return fn;
}
function readSignal() {
  if (this.sources && this.state) {
    if (this.state === STALE) updateComputation(this);
    else {
      const updates = Updates;
      Updates = null;
      runUpdates(() => lookUpstream(this), false);
      Updates = updates;
    }
  }
  if (Listener) {
    const sSlot = this.observers ? this.observers.length : 0;
    if (!Listener.sources) {
      Listener.sources = [this];
      Listener.sourceSlots = [sSlot];
    } else {
      Listener.sources.push(this);
      Listener.sourceSlots.push(sSlot);
    }
    if (!this.observers) {
      this.observers = [Listener];
      this.observerSlots = [Listener.sources.length - 1];
    } else {
      this.observers.push(Listener);
      this.observerSlots.push(Listener.sources.length - 1);
    }
  }
  return this.value;
}
function writeSignal(node, value, isComp) {
  let current = node.value;
  if (!node.comparator || !node.comparator(current, value)) {
    node.value = value;
    if (node.observers && node.observers.length) {
      runUpdates(() => {
        for (let i = 0; i < node.observers.length; i += 1) {
          const o = node.observers[i];
          const TransitionRunning = Transition && Transition.running;
          if (TransitionRunning && Transition.disposed.has(o)) ;
          if (TransitionRunning ? !o.tState : !o.state) {
            if (o.pure) Updates.push(o);
            else Effects.push(o);
            if (o.observers) markDownstream(o);
          }
          if (!TransitionRunning) o.state = STALE;
        }
        if (Updates.length > 1e6) {
          Updates = [];
          if (IS_DEV) ;
          throw new Error();
        }
      }, false);
    }
  }
  return value;
}
function updateComputation(node) {
  if (!node.fn) return;
  cleanNode(node);
  const time = ExecCount;
  runComputation(node, node.value, time);
}
function runComputation(node, value, time) {
  let nextValue;
  const owner = Owner, listener = Listener;
  Listener = Owner = node;
  try {
    nextValue = node.fn(value);
  } catch (err) {
    if (node.pure) {
      {
        node.state = STALE;
        node.owned && node.owned.forEach(cleanNode);
        node.owned = null;
      }
    }
    node.updatedAt = time + 1;
    return handleError(err);
  } finally {
    Listener = listener;
    Owner = owner;
  }
  if (!node.updatedAt || node.updatedAt <= time) {
    if (node.updatedAt != null && "observers" in node) {
      writeSignal(node, nextValue);
    } else node.value = nextValue;
    node.updatedAt = time;
  }
}
function createComputation(fn, init, pure, state2 = STALE, options) {
  const c = {
    fn,
    state: state2,
    updatedAt: null,
    owned: null,
    sources: null,
    sourceSlots: null,
    cleanups: null,
    value: init,
    owner: Owner,
    context: Owner ? Owner.context : null,
    pure
  };
  if (Owner === null) ;
  else if (Owner !== UNOWNED) {
    {
      if (!Owner.owned) Owner.owned = [c];
      else Owner.owned.push(c);
    }
  }
  return c;
}
function runTop(node) {
  if (node.state === 0) return;
  if (node.state === PENDING) return lookUpstream(node);
  if (node.suspense && untrack(node.suspense.inFallback)) return node.suspense.effects.push(node);
  const ancestors = [node];
  while ((node = node.owner) && (!node.updatedAt || node.updatedAt < ExecCount)) {
    if (node.state) ancestors.push(node);
  }
  for (let i = ancestors.length - 1; i >= 0; i--) {
    node = ancestors[i];
    if (node.state === STALE) {
      updateComputation(node);
    } else if (node.state === PENDING) {
      const updates = Updates;
      Updates = null;
      runUpdates(() => lookUpstream(node, ancestors[0]), false);
      Updates = updates;
    }
  }
}
function runUpdates(fn, init) {
  if (Updates) return fn();
  let wait = false;
  if (!init) Updates = [];
  if (Effects) wait = true;
  else Effects = [];
  ExecCount++;
  try {
    const res = fn();
    completeUpdates(wait);
    return res;
  } catch (err) {
    if (!wait) Effects = null;
    Updates = null;
    handleError(err);
  }
}
function completeUpdates(wait) {
  if (Updates) {
    runQueue(Updates);
    Updates = null;
  }
  if (wait) return;
  const e = Effects;
  Effects = null;
  if (e.length) runUpdates(() => runEffects(e), false);
}
function runQueue(queue) {
  for (let i = 0; i < queue.length; i++) runTop(queue[i]);
}
function runUserEffects(queue) {
  let i, userLength = 0;
  for (i = 0; i < queue.length; i++) {
    const e = queue[i];
    if (!e.user) runTop(e);
    else queue[userLength++] = e;
  }
  for (i = 0; i < userLength; i++) runTop(queue[i]);
}
function lookUpstream(node, ignore) {
  node.state = 0;
  for (let i = 0; i < node.sources.length; i += 1) {
    const source = node.sources[i];
    if (source.sources) {
      const state2 = source.state;
      if (state2 === STALE) {
        if (source !== ignore && (!source.updatedAt || source.updatedAt < ExecCount)) runTop(source);
      } else if (state2 === PENDING) lookUpstream(source, ignore);
    }
  }
}
function markDownstream(node) {
  for (let i = 0; i < node.observers.length; i += 1) {
    const o = node.observers[i];
    if (!o.state) {
      o.state = PENDING;
      if (o.pure) Updates.push(o);
      else Effects.push(o);
      o.observers && markDownstream(o);
    }
  }
}
function cleanNode(node) {
  let i;
  if (node.sources) {
    while (node.sources.length) {
      const source = node.sources.pop(), index = node.sourceSlots.pop(), obs = source.observers;
      if (obs && obs.length) {
        const n = obs.pop(), s = source.observerSlots.pop();
        if (index < obs.length) {
          n.sourceSlots[s] = index;
          obs[index] = n;
          source.observerSlots[index] = s;
        }
      }
    }
  }
  if (node.tOwned) {
    for (i = node.tOwned.length - 1; i >= 0; i--) cleanNode(node.tOwned[i]);
    delete node.tOwned;
  }
  if (node.owned) {
    for (i = node.owned.length - 1; i >= 0; i--) cleanNode(node.owned[i]);
    node.owned = null;
  }
  if (node.cleanups) {
    for (i = node.cleanups.length - 1; i >= 0; i--) node.cleanups[i]();
    node.cleanups = null;
  }
  node.state = 0;
}
function castError(err) {
  if (err instanceof Error) return err;
  return new Error(typeof err === "string" ? err : "Unknown error", {
    cause: err
  });
}
function handleError(err, owner = Owner) {
  const error = castError(err);
  throw error;
}
const FALLBACK = /* @__PURE__ */ Symbol("fallback");
function dispose$1(d) {
  for (let i = 0; i < d.length; i++) d[i]();
}
function mapArray(list, mapFn, options = {}) {
  let items = [], mapped = [], disposers = [], len = 0, indexes = mapFn.length > 1 ? [] : null;
  onCleanup(() => dispose$1(disposers));
  return () => {
    let newItems = list() || [], newLen = newItems.length, i, j;
    newItems[$TRACK];
    return untrack(() => {
      let newIndices, newIndicesNext, temp, tempdisposers, tempIndexes, start, end, newEnd, item;
      if (newLen === 0) {
        if (len !== 0) {
          dispose$1(disposers);
          disposers = [];
          items = [];
          mapped = [];
          len = 0;
          indexes && (indexes = []);
        }
        if (options.fallback) {
          items = [FALLBACK];
          mapped[0] = createRoot((disposer) => {
            disposers[0] = disposer;
            return options.fallback();
          });
          len = 1;
        }
      } else if (len === 0) {
        mapped = new Array(newLen);
        for (j = 0; j < newLen; j++) {
          items[j] = newItems[j];
          mapped[j] = createRoot(mapper);
        }
        len = newLen;
      } else {
        temp = new Array(newLen);
        tempdisposers = new Array(newLen);
        indexes && (tempIndexes = new Array(newLen));
        for (start = 0, end = Math.min(len, newLen); start < end && items[start] === newItems[start]; start++) ;
        for (end = len - 1, newEnd = newLen - 1; end >= start && newEnd >= start && items[end] === newItems[newEnd]; end--, newEnd--) {
          temp[newEnd] = mapped[end];
          tempdisposers[newEnd] = disposers[end];
          indexes && (tempIndexes[newEnd] = indexes[end]);
        }
        newIndices = /* @__PURE__ */ new Map();
        newIndicesNext = new Array(newEnd + 1);
        for (j = newEnd; j >= start; j--) {
          item = newItems[j];
          i = newIndices.get(item);
          newIndicesNext[j] = i === void 0 ? -1 : i;
          newIndices.set(item, j);
        }
        for (i = start; i <= end; i++) {
          item = items[i];
          j = newIndices.get(item);
          if (j !== void 0 && j !== -1) {
            temp[j] = mapped[i];
            tempdisposers[j] = disposers[i];
            indexes && (tempIndexes[j] = indexes[i]);
            j = newIndicesNext[j];
            newIndices.set(item, j);
          } else disposers[i]();
        }
        for (j = start; j < newLen; j++) {
          if (j in temp) {
            mapped[j] = temp[j];
            disposers[j] = tempdisposers[j];
            if (indexes) {
              indexes[j] = tempIndexes[j];
              indexes[j](j);
            }
          } else mapped[j] = createRoot(mapper);
        }
        mapped = mapped.slice(0, len = newLen);
        items = newItems.slice(0);
      }
      return mapped;
    });
    function mapper(disposer) {
      disposers[j] = disposer;
      if (indexes) {
        const [s, set] = createSignal(j);
        indexes[j] = set;
        return mapFn(newItems[j], s);
      }
      return mapFn(newItems[j]);
    }
  };
}
function createComponent(Comp, props) {
  return untrack(() => Comp(props || {}));
}
const narrowedError = (name) => `Stale read from <${name}>.`;
function For(props) {
  const fallback = "fallback" in props && {
    fallback: () => props.fallback
  };
  return createMemo(mapArray(() => props.each, props.children, fallback || void 0));
}
function Show(props) {
  const keyed = props.keyed;
  const conditionValue = createMemo(() => props.when, void 0, void 0);
  const condition = keyed ? conditionValue : createMemo(conditionValue, void 0, {
    equals: (a, b) => !a === !b
  });
  return createMemo(() => {
    const c = condition();
    if (c) {
      const child = props.children;
      const fn = typeof child === "function" && child.length > 0;
      return fn ? untrack(() => child(keyed ? c : () => {
        if (!untrack(condition)) throw narrowedError("Show");
        return conditionValue();
      })) : child;
    }
    return props.fallback;
  }, void 0, void 0);
}
const memo = (fn) => createMemo(() => fn());
function reconcileArrays(parentNode, a, b) {
  let bLength = b.length, aEnd = a.length, bEnd = bLength, aStart = 0, bStart = 0, after = a[aEnd - 1].nextSibling, map = null;
  while (aStart < aEnd || bStart < bEnd) {
    if (a[aStart] === b[bStart]) {
      aStart++;
      bStart++;
      continue;
    }
    while (a[aEnd - 1] === b[bEnd - 1]) {
      aEnd--;
      bEnd--;
    }
    if (aEnd === aStart) {
      const node = bEnd < bLength ? bStart ? b[bStart - 1].nextSibling : b[bEnd - bStart] : after;
      while (bStart < bEnd) parentNode.insertBefore(b[bStart++], node);
    } else if (bEnd === bStart) {
      while (aStart < aEnd) {
        if (!map || !map.has(a[aStart])) a[aStart].remove();
        aStart++;
      }
    } else if (a[aStart] === b[bEnd - 1] && b[bStart] === a[aEnd - 1]) {
      const node = a[--aEnd].nextSibling;
      parentNode.insertBefore(b[bStart++], a[aStart++].nextSibling);
      parentNode.insertBefore(b[--bEnd], node);
      a[aEnd] = b[bEnd];
    } else {
      if (!map) {
        map = /* @__PURE__ */ new Map();
        let i = bStart;
        while (i < bEnd) map.set(b[i], i++);
      }
      const index = map.get(a[aStart]);
      if (index != null) {
        if (bStart < index && index < bEnd) {
          let i = aStart, sequence = 1, t;
          while (++i < aEnd && i < bEnd) {
            if ((t = map.get(a[i])) == null || t !== index + sequence) break;
            sequence++;
          }
          if (sequence > index - bStart) {
            const node = a[aStart];
            while (bStart < index) parentNode.insertBefore(b[bStart++], node);
          } else parentNode.replaceChild(b[bStart++], a[aStart++]);
        } else aStart++;
      } else a[aStart++].remove();
    }
  }
}
const $$EVENTS = "_$DX_DELEGATE";
function render$1(code, element, init, options = {}) {
  let disposer;
  createRoot((dispose2) => {
    disposer = dispose2;
    element === document ? code() : insert(element, code(), element.firstChild ? null : void 0, init);
  }, options.owner);
  return () => {
    disposer();
    element.textContent = "";
  };
}
function template(html, isImportNode, isSVG, isMathML) {
  let node;
  const create = () => {
    const t = document.createElement("template");
    t.innerHTML = html;
    return t.content.firstChild;
  };
  const fn = () => (node || (node = create())).cloneNode(true);
  fn.cloneNode = fn;
  return fn;
}
function delegateEvents(eventNames, document2 = window.document) {
  const e = document2[$$EVENTS] || (document2[$$EVENTS] = /* @__PURE__ */ new Set());
  for (let i = 0, l = eventNames.length; i < l; i++) {
    const name = eventNames[i];
    if (!e.has(name)) {
      e.add(name);
      document2.addEventListener(name, eventHandler);
    }
  }
}
function setAttribute(node, name, value) {
  if (value == null) node.removeAttribute(name);
  else node.setAttribute(name, value);
}
function className(node, value) {
  if (value == null) node.removeAttribute("class");
  else node.className = value;
}
function style(node, value, prev) {
  if (!value) return prev ? setAttribute(node, "style") : value;
  const nodeStyle = node.style;
  if (typeof value === "string") return nodeStyle.cssText = value;
  typeof prev === "string" && (nodeStyle.cssText = prev = void 0);
  prev || (prev = {});
  value || (value = {});
  let v, s;
  for (s in prev) {
    value[s] == null && nodeStyle.removeProperty(s);
    delete prev[s];
  }
  for (s in value) {
    v = value[s];
    if (v !== prev[s]) {
      nodeStyle.setProperty(s, v);
      prev[s] = v;
    }
  }
  return prev;
}
function setStyleProperty(node, name, value) {
  value != null ? node.style.setProperty(name, value) : node.style.removeProperty(name);
}
function use(fn, element, arg) {
  return untrack(() => fn(element, arg));
}
function insert(parent, accessor, marker, initial) {
  if (marker !== void 0 && !initial) initial = [];
  if (typeof accessor !== "function") return insertExpression(parent, accessor, initial, marker);
  createRenderEffect((current) => insertExpression(parent, accessor(), current, marker), initial);
}
function eventHandler(e) {
  let node = e.target;
  const key = `$$${e.type}`;
  const oriTarget = e.target;
  const oriCurrentTarget = e.currentTarget;
  const retarget = (value) => Object.defineProperty(e, "target", {
    configurable: true,
    value
  });
  const handleNode = () => {
    const handler = node[key];
    if (handler && !node.disabled) {
      const data = node[`${key}Data`];
      data !== void 0 ? handler.call(node, data, e) : handler.call(node, e);
      if (e.cancelBubble) return;
    }
    node.host && typeof node.host !== "string" && !node.host._$host && node.contains(e.target) && retarget(node.host);
    return true;
  };
  const walkUpTree = () => {
    while (handleNode() && (node = node._$host || node.parentNode || node.host)) ;
  };
  Object.defineProperty(e, "currentTarget", {
    configurable: true,
    get() {
      return node || document;
    }
  });
  if (e.composedPath) {
    const path = e.composedPath();
    retarget(path[0]);
    for (let i = 0; i < path.length - 2; i++) {
      node = path[i];
      if (!handleNode()) break;
      if (node._$host) {
        node = node._$host;
        walkUpTree();
        break;
      }
      if (node.parentNode === oriCurrentTarget) {
        break;
      }
    }
  } else walkUpTree();
  retarget(oriTarget);
}
function insertExpression(parent, value, current, marker, unwrapArray) {
  while (typeof current === "function") current = current();
  if (value === current) return current;
  const t = typeof value, multi = marker !== void 0;
  parent = multi && current[0] && current[0].parentNode || parent;
  if (t === "string" || t === "number") {
    if (t === "number") {
      value = value.toString();
      if (value === current) return current;
    }
    if (multi) {
      let node = current[0];
      if (node && node.nodeType === 3) {
        node.data !== value && (node.data = value);
      } else node = document.createTextNode(value);
      current = cleanChildren(parent, current, marker, node);
    } else {
      if (current !== "" && typeof current === "string") {
        current = parent.firstChild.data = value;
      } else current = parent.textContent = value;
    }
  } else if (value == null || t === "boolean") {
    current = cleanChildren(parent, current, marker);
  } else if (t === "function") {
    createRenderEffect(() => {
      let v = value();
      while (typeof v === "function") v = v();
      current = insertExpression(parent, v, current, marker);
    });
    return () => current;
  } else if (Array.isArray(value)) {
    const array = [];
    const currentArray = current && Array.isArray(current);
    if (normalizeIncomingArray(array, value, current, unwrapArray)) {
      createRenderEffect(() => current = insertExpression(parent, array, current, marker, true));
      return () => current;
    }
    if (array.length === 0) {
      current = cleanChildren(parent, current, marker);
      if (multi) return current;
    } else if (currentArray) {
      if (current.length === 0) {
        appendNodes(parent, array, marker);
      } else reconcileArrays(parent, current, array);
    } else {
      current && cleanChildren(parent);
      appendNodes(parent, array);
    }
    current = array;
  } else if (value.nodeType) {
    if (Array.isArray(current)) {
      if (multi) return current = cleanChildren(parent, current, marker, value);
      cleanChildren(parent, current, null, value);
    } else if (current == null || current === "" || !parent.firstChild) {
      parent.appendChild(value);
    } else parent.replaceChild(value, parent.firstChild);
    current = value;
  } else ;
  return current;
}
function normalizeIncomingArray(normalized, array, current, unwrap) {
  let dynamic = false;
  for (let i = 0, len = array.length; i < len; i++) {
    let item = array[i], prev = current && current[normalized.length], t;
    if (item == null || item === true || item === false) ;
    else if ((t = typeof item) === "object" && item.nodeType) {
      normalized.push(item);
    } else if (Array.isArray(item)) {
      dynamic = normalizeIncomingArray(normalized, item, prev) || dynamic;
    } else if (t === "function") {
      if (unwrap) {
        while (typeof item === "function") item = item();
        dynamic = normalizeIncomingArray(normalized, Array.isArray(item) ? item : [item], Array.isArray(prev) ? prev : [prev]) || dynamic;
      } else {
        normalized.push(item);
        dynamic = true;
      }
    } else {
      const value = String(item);
      if (prev && prev.nodeType === 3 && prev.data === value) normalized.push(prev);
      else normalized.push(document.createTextNode(value));
    }
  }
  return dynamic;
}
function appendNodes(parent, array, marker = null) {
  for (let i = 0, len = array.length; i < len; i++) parent.insertBefore(array[i], marker);
}
function cleanChildren(parent, current, marker, replacement) {
  if (marker === void 0) return parent.textContent = "";
  const node = replacement || document.createTextNode("");
  if (current.length) {
    let inserted = false;
    for (let i = current.length - 1; i >= 0; i--) {
      const el = current[i];
      if (node !== el) {
        const isParent = el.parentNode === parent;
        if (!inserted && !i) isParent ? parent.replaceChild(node, el) : parent.insertBefore(node, marker);
        else isParent && el.remove();
      } else inserted = true;
    }
  } else parent.insertBefore(node, marker);
  return [node];
}
const TEST_API_GLOBAL_NAME = "__AW_TEST__";
const RENDER_META_GLOBAL_NAME = "__AW_VIEWER_RENDER_META__";
const VIEWER_RENDER_MODE = "viewer";
const SOFTWARE_SAFE_RENDER_MODE_ALIAS = "software_safe";
const VIEWER_AUTH_BOOTSTRAP_OBJECT = "__OASIS7_VIEWER_AUTH_ENV";
const VIEWER_PLAYER_ID_KEY = "OASIS7_VIEWER_PLAYER_ID";
const VIEWER_AUTH_PUBLIC_KEY = "OASIS7_VIEWER_AUTH_PUBLIC_KEY";
const VIEWER_AUTH_PRIVATE_KEY = "OASIS7_VIEWER_AUTH_PRIVATE_KEY";
const VIEWER_AUTH_SIGNATURE_PREFIX = "awviewauth:v1:";
const HOSTED_PLAYER_SESSION_STORAGE_PREFIX = "oasis7.hosted_player_session.v1";
const UI_LOCALE_STORAGE_PREFIX = "oasis7.viewer.locale.v1";
const PROMPT_OVERRIDES_VISIBILITY_STORAGE_PREFIX = "oasis7.viewer.prompt_overrides_visible.v1";
const HOSTED_PLAYER_SESSION_ADMISSION_ROUTE = "/api/public/player-session/admission";
const HOSTED_PLAYER_SESSION_REFRESH_ROUTE = "/api/public/player-session/refresh";
const HOSTED_PLAYER_SESSION_ISSUE_ROUTE = "/api/public/player-session/issue";
const HOSTED_PLAYER_SESSION_RELEASE_ROUTE = "/api/public/player-session/release";
const HOSTED_STRONG_AUTH_GRANT_ROUTE = "/api/public/strong-auth/grant";
const HOSTED_PLAYER_SESSION_REFRESH_INTERVAL_MS = 3e4;
const DEFAULT_WS_ADDR = "ws://127.0.0.1:5011";
const MAX_EVENTS = 24;
const SOFTWARE_RENDERER_MARKERS = [
  "swiftshader",
  "llvmpipe",
  "software rasterizer",
  "basic render driver",
  "softpipe",
  "lavapipe"
];
const ED25519_PKCS8_PREFIX = new Uint8Array([
  48,
  46,
  2,
  1,
  0,
  48,
  5,
  6,
  3,
  43,
  101,
  112,
  4,
  34,
  4,
  32
]);
const textEncoder = new TextEncoder();
const state = {
  uiLocale: "en",
  promptOverridesVisible: false,
  connectionStatus: "connecting",
  logicalTime: 0,
  eventSeq: 0,
  tick: 0,
  selectedKind: null,
  selectedId: null,
  errorCount: 0,
  lastError: null,
  eventCount: 0,
  traceCount: 0,
  cameraMode: "viewer",
  cameraRadius: 0,
  cameraOrthoScale: 0,
  renderMode: VIEWER_RENDER_MODE,
  rendererClass: "none",
  viewerReason: null,
  renderer: null,
  vendor: null,
  webglVersion: null,
  pixelWorldRuntimeStatus: "detached",
  pixelWorldRuntimeSource: "detached",
  pixelWorldRuntimeModuleUrl: null,
  pixelWorldCamera: null,
  pixelWorldFatal: null,
  controlProfile: "playback",
  debugViewerMode: "debug_viewer",
  debugViewerStatus: "detached",
  worldId: null,
  server: null,
  wsUrl: null,
  lastControlFeedback: null,
  lastPromptFeedback: null,
  lastChatFeedback: null,
  lastGameplayActionFeedback: null,
  snapshot: null,
  metrics: null,
  hostedAccess: null,
  hostedAdmission: null,
  recentEvents: [],
  chatHistory: [],
  selectedObject: null,
  auth: {
    available: false,
    playerId: null,
    publicKey: null,
    privateKey: null,
    releaseToken: null,
    error: null,
    revokeReason: null,
    revokedBy: null,
    source: "guest_only",
    registrationStatus: "guest",
    sessionEpoch: null,
    issuedAtUnixMs: null,
    recoveryErrorCode: null,
    recoveryErrorMessage: null,
    issueInFlight: false,
    syncInFlight: false,
    runtimeStatus: "guest",
    boundAgentId: null,
    pendingRequestedAgentId: null,
    pendingForceRebind: false,
    rebindNotice: null
  },
  promptDraft: {
    agentId: null,
    currentVersion: 0,
    rollbackTargetVersion: 0,
    updatedBy: "",
    updatedAtTick: 0,
    systemPrompt: "",
    shortTermGoal: "",
    longTermGoal: "",
    dirty: false
  },
  chatDraft: {
    agentId: null,
    message: "",
    dirty: false
  },
  strongAuth: {
    approvalCode: "",
    lastGrantActionId: null,
    lastGrantExpiresAtUnixMs: null,
    lastGrantError: null
  }
};
let socket = null;
let reconnectTimer = null;
let hostedSessionRefreshTimer = null;
let requestId = 0;
let authNonceCounter = 0;
let selectedSearch = "";
let semanticSendLoop = null;
const pendingControlFeedback = /* @__PURE__ */ new Map();
const pendingSemanticCommands = [];
const authKeyCache = /* @__PURE__ */ new Map();
let pendingSessionRegisterWaiter = null;
let renderHook = () => {
};
let bootstrapped = false;
function normalizeUiLocale(raw) {
  const value = String(raw || "").trim().toLowerCase();
  if (["zh", "zh-cn", "zh_cn", "cn", "chinese"].includes(value)) {
    return "zh";
  }
  if (["en", "en-us", "en_us", "english"].includes(value)) {
    return "en";
  }
  return null;
}
function isLocaleZh(locale = state.uiLocale) {
  return normalizeUiLocale(locale) === "zh";
}
function uiLocaleStorageKey() {
  return `${UI_LOCALE_STORAGE_PREFIX}:${window.location.pathname || "viewer.html"}`;
}
function persistUiLocale(locale) {
  try {
    window.localStorage?.setItem(uiLocaleStorageKey(), locale);
  } catch (_) {
  }
}
function resolveStoredUiLocale() {
  try {
    return normalizeUiLocale(window.localStorage?.getItem(uiLocaleStorageKey()));
  } catch (_) {
    return null;
  }
}
function resolveInitialUiLocale() {
  const params = getSearchParams();
  return normalizeUiLocale(params.get("locale") || params.get("language")) || resolveStoredUiLocale() || "en";
}
function promptOverridesVisibilityStorageKey() {
  return `${PROMPT_OVERRIDES_VISIBILITY_STORAGE_PREFIX}:${window.location.pathname || "viewer.html"}`;
}
function persistPromptOverridesVisibility(visible) {
  try {
    window.localStorage?.setItem(promptOverridesVisibilityStorageKey(), visible ? "1" : "0");
  } catch (_) {
  }
}
function resolveStoredPromptOverridesVisibility() {
  try {
    const raw = window.localStorage?.getItem(promptOverridesVisibilityStorageKey());
    return raw === "1";
  } catch (_) {
    return false;
  }
}
function applyUiLocaleToDocument(locale) {
  document.documentElement.lang = locale === "zh" ? "zh-CN" : "en";
}
function updateUiLocaleQuery(locale) {
  const url = new URL(window.location.href);
  url.searchParams.set("locale", locale);
  url.searchParams.delete("language");
  window.history.replaceState({}, "", url.toString());
}
function setViewerLocale(locale) {
  const normalized = normalizeUiLocale(locale);
  if (!normalized) {
    return state.uiLocale;
  }
  state.uiLocale = normalized;
  persistUiLocale(normalized);
  applyUiLocaleToDocument(normalized);
  updateUiLocaleQuery(normalized);
  render();
  return state.uiLocale;
}
function setPromptOverridesVisible(visible) {
  state.promptOverridesVisible = !!visible;
  persistPromptOverridesVisibility(state.promptOverridesVisible);
  render();
  return state.promptOverridesVisible;
}
function togglePromptOverridesVisible() {
  return setPromptOverridesVisible(!state.promptOverridesVisible);
}
function getSelectedSearch() {
  return selectedSearch;
}
function setSelectedSearch(value) {
  selectedSearch = String(value || "");
  render();
}
function setRenderHook(nextHook) {
  renderHook = typeof nextHook === "function" ? nextHook : () => {
  };
}
function getSearchParams() {
  return new URLSearchParams(window.location.search || "");
}
function isTestApiEnabled() {
  const value = String(getSearchParams().get("test_api") || "").trim().toLowerCase();
  return value === "1" || value === "true" || value === "yes" || value === "on";
}
function normalizeWsAddr(raw) {
  const value = String(raw || "").trim();
  if (!value) return DEFAULT_WS_ADDR;
  if (value.startsWith("ws://") || value.startsWith("wss://")) return value;
  if (value.startsWith("http://")) return `ws://${value.slice("http://".length)}`;
  if (value.startsWith("https://")) return `wss://${value.slice("https://".length)}`;
  return `ws://${value}`;
}
function clone(value) {
  return value == null ? value : JSON.parse(JSON.stringify(value));
}
function normalizeFiniteNumber(value) {
  if (value == null) {
    return null;
  }
  const numeric = Number(value);
  return Number.isFinite(numeric) ? numeric : null;
}
function finitePositionComponents(pos) {
  if (!pos || typeof pos !== "object") {
    return null;
  }
  const x = normalizeFiniteNumber(pos.x_cm);
  const y = normalizeFiniteNumber(pos.y_cm);
  const z = normalizeFiniteNumber(pos.z_cm);
  if (x == null || y == null || z == null) {
    return null;
  }
  return { x, y, z };
}
function trimFixed(value, digits) {
  if (!Number.isFinite(value)) {
    return null;
  }
  const fixed = value.toFixed(digits);
  return fixed.replace(/\.0+$/, "").replace(/(\.\d*[1-9])0+$/, "$1");
}
function formatPhysicalDistanceCm(value, locale = state.uiLocale) {
  const numeric = normalizeFiniteNumber(value);
  if (numeric == null) {
    return null;
  }
  const absolute = Math.abs(numeric);
  if (absolute >= 1e5) {
    const km = numeric / 1e5;
    const label = trimFixed(km, Math.abs(km) >= 100 ? 0 : Math.abs(km) >= 10 ? 1 : 2);
    return `${label} km`;
  }
  if (absolute >= 100) {
    const meters = numeric / 100;
    const label = trimFixed(
      meters,
      Math.abs(meters) >= 100 ? 0 : Math.abs(meters) >= 10 ? 1 : 2
    );
    return `${label} m`;
  }
  return `${trimFixed(numeric, 0)} cm`;
}
function formatWorldPositionCm(pos, locale = state.uiLocale) {
  if (!pos || typeof pos !== "object") {
    return null;
  }
  const x = formatPhysicalDistanceCm(pos.x_cm, locale);
  const y = formatPhysicalDistanceCm(pos.y_cm, locale);
  const z = formatPhysicalDistanceCm(pos.z_cm, locale);
  if (!x || !y || !z) {
    return null;
  }
  return `x=${x} · y=${y} · z=${z}`;
}
function distanceCmBetweenPositions(a, b) {
  const left = finitePositionComponents(a);
  const right = finitePositionComponents(b);
  if (!left || !right) {
    return null;
  }
  const dx = left.x - right.x;
  const dy = left.y - right.y;
  const dz = left.z - right.z;
  return Math.max(0, Math.round(Math.sqrt(dx * dx + dy * dy + dz * dz)));
}
function locationRadiusCm(location) {
  return normalizeFiniteNumber(location?.profile?.radius_cm);
}
function snapshotSpaceConfig() {
  const space = state.snapshot?.config?.space;
  return space && typeof space === "object" ? space : null;
}
function selectedWorldAnchor() {
  const selected = state.selectedObject;
  if (selected && selected.pos) {
    return {
      kind: state.selectedKind || "location",
      id: state.selectedId || selected.id || selected.name || "selected",
      pos: selected.pos,
      radiusCm: locationRadiusCm(selected),
      locationId: selected.location_id || selected.id || null
    };
  }
  const locations = Object.values(state.snapshot?.model?.locations || {});
  const fallback = locations.find((location) => location?.pos);
  if (!fallback) {
    return null;
  }
  return {
    kind: "location",
    id: fallback.id || fallback.name || "location",
    pos: fallback.pos,
    radiusCm: locationRadiusCm(fallback),
    locationId: fallback.id || null
  };
}
function buildWorldScaleSurface(locale = state.uiLocale) {
  const isZh = isLocaleZh(locale);
  const space = snapshotSpaceConfig();
  const anchor = selectedWorldAnchor();
  const locations = Object.values(state.snapshot?.model?.locations || {}).filter((location) => location?.id && location?.pos);
  const nearestLocations = anchor ? locations.filter((location) => location.id !== anchor.locationId).map((location) => {
    const distanceCm = distanceCmBetweenPositions(anchor.pos, location.pos);
    return {
      id: location.id,
      name: location.name || location.id,
      distanceCm,
      distanceLabel: formatPhysicalDistanceCm(distanceCm, locale),
      radiusCm: locationRadiusCm(location),
      radiusLabel: formatPhysicalDistanceCm(locationRadiusCm(location), locale)
    };
  }).filter((location) => location.distanceCm != null).sort((left, right) => left.distanceCm - right.distanceCm).slice(0, 3) : [];
  const physicalTruth = {
    canonicalUnitLabel: formatPhysicalDistanceCm(1, locale),
    canonicalUnitDetail: isZh ? "世界位置、距离、半径和尺寸的正式真值都按整数厘米存储。" : "World positions, distances, radii, and sizes are stored as integer centimeters.",
    worldBoundsLabel: space ? `${formatPhysicalDistanceCm(space.width_cm, locale)} × ${formatPhysicalDistanceCm(space.depth_cm, locale)} × ${formatPhysicalDistanceCm(space.height_cm, locale)}` : null,
    worldBoundsDetail: space ? isZh ? "来自 snapshot.config.space 的真实世界边界。" : "Physical world bounds derived from snapshot.config.space." : isZh ? "当前快照没有发布 world bounds。" : "The current snapshot does not publish world bounds yet.",
    anchor: anchor ? {
      kind: anchor.kind,
      id: anchor.id,
      label: anchor.kind === "agent" ? isZh ? "当前选中 Agent 锚点" : "Selected agent anchor" : isZh ? "当前选中地点锚点" : "Selected location anchor",
      positionLabel: formatWorldPositionCm(anchor.pos, locale),
      radiusCm: anchor.radiusCm,
      radiusLabel: anchor.radiusCm == null ? null : formatPhysicalDistanceCm(anchor.radiusCm, locale),
      locationId: anchor.locationId
    } : null,
    nearestLocations
  };
  const presentationScale = {
    markerTruthNote: isZh ? "3D marker、2D overview map 和 halo 允许为了可读性被放大；请把距离/半径标签当成真值，不要把屏幕上的直径当成真实几何尺寸。" : "3D markers, the 2D overview map, and halos may be enlarged for readability. Treat the distance/radius labels as truth; do not read on-screen diameter as real geometry size.",
    zoomTruthNote: isZh ? "overview/detail 的 zoom tier 只切换表现语义，不会改写世界的厘米真值。" : "Overview/detail zoom tiers only switch presentation semantics; they do not rewrite centimeter truth in the world model.",
    softwareSafeNote: isZh ? "viewer 主入口优先给出文字和数值真值；更底层的 visual QA viewer 可以更夸张，但不应覆盖这里的物理标签。" : "The viewer entry prioritizes textual and numeric truth. Lower-level visual QA surfaces may exaggerate more aggressively, but they should not override the physical labels here."
  };
  return {
    physicalTruth,
    presentationScale
  };
}
function detectRendererMeta() {
  const params = getSearchParams();
  const reasonFromQuery = params.get("viewer_reason") || params.get("software_safe_reason");
  const requestedRenderMode = String(params.get("render_mode") || "").trim().toLowerCase();
  const meta = {
    renderMode: requestedRenderMode === SOFTWARE_SAFE_RENDER_MODE_ALIAS || requestedRenderMode === VIEWER_RENDER_MODE ? VIEWER_RENDER_MODE : VIEWER_RENDER_MODE,
    rendererClass: "none",
    viewerReason: reasonFromQuery || "direct_viewer_entry",
    renderer: null,
    vendor: null,
    webglVersion: null
  };
  try {
    const canvas = document.createElement("canvas");
    const gl = canvas.getContext("webgl") || canvas.getContext("experimental-webgl");
    if (!gl) {
      meta.rendererClass = "none";
      meta.viewerReason = reasonFromQuery || "webgl_unavailable";
      return meta;
    }
    meta.webglVersion = gl.getParameter(gl.VERSION) || null;
    const debugInfo = gl.getExtension("WEBGL_debug_renderer_info");
    if (debugInfo) {
      meta.renderer = gl.getParameter(debugInfo.UNMASKED_RENDERER_WEBGL) || null;
      meta.vendor = gl.getParameter(debugInfo.UNMASKED_VENDOR_WEBGL) || null;
    }
    const rendererText = String(meta.renderer || "").toLowerCase();
    if (SOFTWARE_RENDERER_MARKERS.some((marker) => rendererText.includes(marker))) {
      meta.rendererClass = "software";
    } else {
      meta.rendererClass = "unknown";
    }
  } catch (error) {
    meta.rendererClass = "none";
    meta.renderer = String(error);
  }
  return meta;
}
function resolveAuthBootstrap() {
  const raw = window[VIEWER_AUTH_BOOTSTRAP_OBJECT];
  if (!raw || typeof raw !== "object") {
    return {
      available: false,
      playerId: null,
      publicKey: null,
      privateKey: null,
      releaseToken: null,
      error: "viewer auth bootstrap is unavailable",
      revokeReason: null,
      revokedBy: null,
      source: "guest_only",
      registrationStatus: "guest",
      sessionEpoch: null,
      issuedAtUnixMs: null,
      recoveryErrorCode: null,
      recoveryErrorMessage: null,
      issueInFlight: false,
      syncInFlight: false,
      runtimeStatus: "guest",
      boundAgentId: null,
      pendingRequestedAgentId: null,
      pendingForceRebind: false,
      rebindNotice: null
    };
  }
  const playerId = String(raw[VIEWER_PLAYER_ID_KEY] || "").trim();
  const publicKey = String(raw[VIEWER_AUTH_PUBLIC_KEY] || "").trim().toLowerCase();
  const privateKey = String(raw[VIEWER_AUTH_PRIVATE_KEY] || "").trim().toLowerCase();
  if (!playerId || !publicKey || !privateKey) {
    return {
      available: false,
      playerId: playerId || null,
      publicKey: publicKey || null,
      privateKey: privateKey || null,
      releaseToken: null,
      error: "viewer auth bootstrap is incomplete",
      revokeReason: null,
      revokedBy: null,
      source: "guest_only",
      registrationStatus: "guest",
      sessionEpoch: null,
      issuedAtUnixMs: null,
      recoveryErrorCode: null,
      recoveryErrorMessage: null,
      issueInFlight: false,
      syncInFlight: false,
      runtimeStatus: "guest",
      boundAgentId: null,
      pendingRequestedAgentId: null,
      pendingForceRebind: false,
      rebindNotice: null
    };
  }
  return {
    available: true,
    playerId,
    publicKey,
    privateKey,
    releaseToken: null,
    error: null,
    revokeReason: null,
    revokedBy: null,
    source: "legacy_viewer_auth_bootstrap",
    registrationStatus: "registered",
    sessionEpoch: 1,
    issuedAtUnixMs: null,
    recoveryErrorCode: null,
    recoveryErrorMessage: null,
    issueInFlight: false,
    syncInFlight: false,
    runtimeStatus: "legacy_preview",
    boundAgentId: null,
    pendingRequestedAgentId: null,
    pendingForceRebind: false,
    rebindNotice: null
  };
}
function initialWsUrl() {
  const params = getSearchParams();
  return normalizeWsAddr(params.get("ws") || params.get("addr") || DEFAULT_WS_ADDR);
}
function shouldConnectViewerWs() {
  const mode = String(getSearchParams().get("connect") || "").trim().toLowerCase();
  return mode !== "0" && mode !== "false" && mode !== "off";
}
function hostedPlayerSessionStorageKey() {
  return `${HOSTED_PLAYER_SESSION_STORAGE_PREFIX}:${initialWsUrl()}`;
}
function persistHostedPlayerSession(auth) {
  if (!auth?.available || !auth?.playerId || !auth?.publicKey || !auth?.privateKey || auth.source === "legacy_viewer_auth_bootstrap") {
    return;
  }
  try {
    window.localStorage?.setItem(
      hostedPlayerSessionStorageKey(),
      JSON.stringify({
        playerId: auth.playerId,
        publicKey: auth.publicKey,
        privateKey: auth.privateKey,
        releaseToken: auth.releaseToken || null,
        issuedAtUnixMs: auth.issuedAtUnixMs || null,
        sessionEpoch: auth.sessionEpoch || null
      })
    );
  } catch (_) {
  }
}
function clearHostedPlayerSession() {
  try {
    window.localStorage?.removeItem(hostedPlayerSessionStorageKey());
  } catch (_) {
  }
}
function resolveStoredHostedPlayerSession() {
  try {
    const raw = window.localStorage?.getItem(hostedPlayerSessionStorageKey());
    if (!raw) {
      return null;
    }
    const parsed = JSON.parse(raw);
    const playerId = String(parsed?.playerId || "").trim();
    const publicKey = String(parsed?.publicKey || "").trim().toLowerCase();
    const privateKey = String(parsed?.privateKey || "").trim().toLowerCase();
    const releaseToken = String(parsed?.releaseToken || "").trim();
    if (!playerId || !publicKey || !privateKey || !releaseToken) {
      clearHostedPlayerSession();
      return null;
    }
    return {
      available: true,
      playerId,
      publicKey,
      privateKey,
      releaseToken,
      error: null,
      revokeReason: null,
      revokedBy: null,
      source: "hosted_browser_storage",
      registrationStatus: "issued",
      sessionEpoch: parsed?.sessionEpoch == null ? null : Number(parsed.sessionEpoch),
      issuedAtUnixMs: parsed?.issuedAtUnixMs == null ? null : Number(parsed.issuedAtUnixMs),
      recoveryErrorCode: null,
      recoveryErrorMessage: null,
      issueInFlight: false,
      syncInFlight: false,
      runtimeStatus: "issued",
      boundAgentId: null,
      pendingRequestedAgentId: null,
      pendingForceRebind: false,
      rebindNotice: null
    };
  } catch (_) {
    clearHostedPlayerSession();
    return null;
  }
}
function resolveViewerAuthState() {
  const bootstrap2 = resolveAuthBootstrap();
  if (bootstrap2.available) {
    return bootstrap2;
  }
  return resolveStoredHostedPlayerSession() || bootstrap2;
}
async function refreshHostedAdmissionState() {
  if (String(state.hostedAccess?.deployment_mode || "").trim() !== "hosted_public_join") {
    state.hostedAdmission = null;
    return null;
  }
  try {
    const response = await fetch(HOSTED_PLAYER_SESSION_ADMISSION_ROUTE, {
      method: "GET",
      cache: "no-store",
      headers: { Accept: "application/json" }
    });
    const payload = await response.json();
    state.hostedAdmission = payload?.admission ? clone(payload.admission) : null;
    return state.hostedAdmission;
  } catch (_) {
    return state.hostedAdmission;
  }
}
async function refreshHostedPlayerLease() {
  const playerId = String(state.auth.playerId || "").trim();
  const releaseToken = String(state.auth.releaseToken || "").trim();
  if (!playerId || !releaseToken || state.auth.source === "legacy_viewer_auth_bootstrap") {
    return null;
  }
  try {
    const response = await fetch(
      `${HOSTED_PLAYER_SESSION_REFRESH_ROUTE}?player_id=${encodeURIComponent(playerId)}&release_token=${encodeURIComponent(releaseToken)}`,
      {
        method: "POST",
        cache: "no-store",
        headers: { Accept: "application/json" }
      }
    );
    const payload = await response.json();
    if (payload?.admission) {
      state.hostedAdmission = clone(payload.admission);
    }
    if (!response.ok || !payload?.ok) {
      throw new Error(payload?.error || payload?.error_code || `hosted player-session refresh failed with HTTP ${response.status}`);
    }
    return payload;
  } catch (error) {
    state.auth.error = String(error);
    return null;
  }
}
function stopHostedSessionRefreshLoop() {
  if (hostedSessionRefreshTimer) {
    window.clearInterval(hostedSessionRefreshTimer);
    hostedSessionRefreshTimer = null;
  }
}
function syncHostedSessionRefreshLoop() {
  const shouldRun = state.connectionStatus === "connected" && state.auth.available && state.auth.source !== "legacy_viewer_auth_bootstrap" && state.auth.registrationStatus === "registered" && !!state.auth.releaseToken;
  if (!shouldRun) {
    stopHostedSessionRefreshLoop();
    return;
  }
  if (hostedSessionRefreshTimer) {
    return;
  }
  hostedSessionRefreshTimer = window.setInterval(() => {
    probeHostedRuntimeSession();
    void refreshHostedPlayerLease().then(() => render());
  }, HOSTED_PLAYER_SESSION_REFRESH_INTERVAL_MS);
}
function resolveHostedAccessHint() {
  const raw = getSearchParams().get("hosted_access");
  if (!raw) {
    return null;
  }
  try {
    const parsed = JSON.parse(raw);
    return parsed && typeof parsed === "object" ? parsed : null;
  } catch (_) {
    return null;
  }
}
function hostnameFromUrl(raw) {
  const value = String(raw || "").trim();
  if (!value) return null;
  try {
    return new URL(value, window.location.href).hostname || null;
  } catch (_) {
    return null;
  }
}
function isLoopbackHostname(raw) {
  const value = String(raw || "").trim().toLowerCase();
  return value === "localhost" || value === "127.0.0.1" || value === "::1" || value === "[::1]";
}
function authDeploymentHint(auth) {
  const hostedMode = String(state.hostedAccess?.deployment_mode || "").trim();
  if (hostedMode === "hosted_public_join") {
    if (auth.available && auth.source === "legacy_viewer_auth_bootstrap") {
      return "hosted_public_join_contract_with_legacy_bootstrap";
    }
    return auth.available ? "hosted_public_join_contract_with_browser_session" : "hosted_public_join_contract";
  }
  if (hostedMode === "trusted_local_only") {
    return auth.available ? "trusted_local_contract" : "trusted_local_contract_guest";
  }
  const params = getSearchParams();
  const wsHost = hostnameFromUrl(state.wsUrl || params.get("ws") || params.get("addr") || "");
  const pageHost = String(window.location.hostname || "").trim();
  const remoteOriginLikely = [pageHost, wsHost].filter(Boolean).some((host) => !isLoopbackHostname(host));
  if (auth.available) {
    return remoteOriginLikely ? "remote_origin_legacy_bootstrap" : "trusted_local_preview";
  }
  return remoteOriginLikely ? "hosted_public_join_likely" : "guest_only_or_missing_bootstrap";
}
function isHostedPublicJoinHint(deploymentHint) {
  return [
    "hosted_public_join_contract",
    "hosted_public_join_contract_with_legacy_bootstrap",
    "hosted_public_join_likely"
  ].includes(deploymentHint);
}
function hostedActionPolicy(actionId) {
  const normalizedActionId = actionId === "prompt_control" ? "prompt_control_apply" : actionId;
  return state.hostedAccess?.action_matrix?.find((policy) => policy?.action_id === normalizedActionId) || null;
}
function guestSessionReason(auth, deploymentHint) {
  if (auth.available) {
    return auth.source === "legacy_viewer_auth_bootstrap" ? "guest session has already been superseded by the current preview player auth lane" : "guest session has already been superseded by a hosted-issued player identity";
  }
  if (isHostedPublicJoinHint(deploymentHint)) {
    return auth.error || "this browser is still guest-only; hosted public join must issue a player identity before low-risk interaction unlocks";
  }
  return auth.error || "viewer auth bootstrap is unavailable, so the browser cannot leave guest session";
}
function playerSessionReason(auth, deploymentHint) {
  if (auth.available) {
    if (auth.source === "legacy_viewer_auth_bootstrap") {
      return "player interaction is currently unlocked through legacy viewer auth bootstrap in trusted preview mode";
    }
    if (auth.registrationStatus === "registered") {
      return "player interaction is unlocked through hosted-issued player_id + browser-local ephemeral Ed25519 session";
    }
    if (auth.registrationStatus === "registering" || auth.registrationStatus === "issued") {
      return "browser-local hosted identity is ready; runtime player-session registration is still in progress";
    }
    return auth.error || "hosted player identity exists, but runtime registration still needs recovery";
  }
  if (isHostedPublicJoinHint(deploymentHint)) {
    return auth.error || "player session upgrade/login is still pending hosted issue";
  }
  return auth.error || "viewer auth bootstrap is missing or incomplete";
}
function strongAuthReason() {
  return "strong auth remains a separate upgrade plane; viewer only previews backend reauth for prompt_control and still does not issue hosted-ready asset/governance proofs";
}
function isStrongAuthSensitiveAction(actionId) {
  const policy = hostedActionPolicy(actionId);
  if (policy) {
    return policy.required_auth === "strong_auth";
  }
  return actionId === "prompt_control" || actionId === "main_token_transfer";
}
function buildSemanticCapability(actionId) {
  const observerOnly = selectedAgentInteractionMode() === "observer_only";
  const deploymentHint = authDeploymentHint(state.auth);
  const strongAuthSensitive = isStrongAuthSensitiveAction(actionId);
  const policy = hostedActionPolicy(actionId);
  if (observerOnly) {
    return {
      actionId,
      enabled: false,
      code: "observer_only",
      reason: "selected agent runs through the provider-backed loopback bridge; viewer stays observer-only for prompt/chat on this lane"
    };
  }
  if (policy) {
    if (policy.required_auth === "strong_auth") {
      const isLocalPreviewOnly = policy.availability === "trusted_local_preview_only";
      const isBackendGrantPreview = policy.availability === "public_player_plane_with_backend_reauth_preview";
      if (isLocalPreviewOnly && state.auth.available && !isHostedPublicJoinHint(deploymentHint)) {
        return {
          actionId,
          enabled: true,
          code: null,
          reason: policy.reason || "trusted local preview currently allows this strong-auth-marked action through preview bootstrap"
        };
      }
      if (isBackendGrantPreview && state.auth.available) {
        return {
          actionId,
          enabled: true,
          code: null,
          reason: policy.reason || `${actionId} is available through browser-local player auth plus backend re-authorization`
        };
      }
      if (isBackendGrantPreview && !state.auth.available) {
        return {
          actionId,
          enabled: false,
          code: "auth_level_insufficient",
          reason: `${actionId} requires player_session before backend re-authorization can upgrade it to strong_auth`
        };
      }
      return {
        actionId,
        enabled: false,
        code: "strong_auth_required",
        reason: policy.reason || strongAuthReason()
      };
    }
    if (!state.auth.available) {
      return {
        actionId,
        enabled: false,
        code: "auth_level_insufficient",
        reason: `${actionId} requires ${policy.required_auth}; current browser remains guest_session only`
      };
    }
    return {
      actionId,
      enabled: true,
      code: null,
      reason: policy.reason || `${actionId} is allowed on the ${policy.required_auth} lane`
    };
  }
  if (strongAuthSensitive && isHostedPublicJoinHint(deploymentHint)) {
    return {
      actionId,
      enabled: false,
      code: "strong_auth_required",
      reason: `${actionId} requires strong_auth on the hosted public join path; this browser is still guest_session only and the strong-auth upgrade lane is not implemented yet`
    };
  }
  if (strongAuthSensitive && state.auth.available && deploymentHint === "remote_origin_legacy_bootstrap") {
    return {
      actionId,
      enabled: false,
      code: "strong_auth_required",
      reason: `${actionId} is blocked on remote-origin legacy bootstrap; hosted/public prompt control must move to strong_auth or private operator plane`
    };
  }
  if (!state.auth.available) {
    const reason = isHostedPublicJoinHint(deploymentHint) ? `${actionId} requires player_session; this browser is still guest_session only on the hosted public join path` : `${actionId} requires viewer auth bootstrap; current status: ${state.auth.error || "missing"}`;
    return {
      actionId,
      enabled: false,
      code: "auth_level_insufficient",
      reason
    };
  }
  return {
    actionId,
    enabled: true,
    code: null,
    reason: strongAuthSensitive ? "prompt_control stays enabled only in trusted_local_preview via legacy viewer auth bootstrap; hosted/public strong_auth remains pending" : "player_session is active via legacy viewer auth bootstrap preview"
  };
}
function buildAuthSurfaceModel() {
  const deploymentHint = authDeploymentHint(state.auth);
  const promptCapability = buildSemanticCapability("prompt_control");
  const chatCapability = buildSemanticCapability("agent_chat");
  const mainTokenTransferCapability = buildSemanticCapability("main_token_transfer");
  const currentTier = state.auth.available ? "player_session" : "guest_session";
  const source = state.hostedAccess ? state.auth.available ? state.auth.source === "legacy_viewer_auth_bootstrap" ? "legacy_viewer_auth_bootstrap+hosted_access_hint" : "hosted_player_issue+browser_local_ephemeral_key" : "hosted_access_hint" : state.auth.available ? state.auth.source : "guest_only";
  return {
    deploymentHint,
    source,
    currentTier,
    currentTierReason: currentTier === "player_session" ? playerSessionReason(state.auth, deploymentHint) : guestSessionReason(state.auth, deploymentHint),
    tiers: [
      {
        id: "guest_session",
        label: "guest_session",
        status: state.auth.available ? "superseded" : "active",
        reason: guestSessionReason(state.auth, deploymentHint)
      },
      {
        id: "player_session",
        label: "player_session",
        status: state.auth.available ? state.auth.source === "legacy_viewer_auth_bootstrap" ? "active_legacy_preview" : state.auth.registrationStatus === "registered" ? "active_hosted_issue" : "issued_pending_register" : "not_issued",
        reason: playerSessionReason(state.auth, deploymentHint)
      },
      {
        id: "strong_auth",
        label: "strong_auth",
        status: "not_implemented",
        reason: strongAuthReason()
      }
    ],
    capabilities: {
      prompt_control: promptCapability,
      agent_chat: chatCapability,
      main_token_transfer: mainTokenTransferCapability,
      strong_auth_actions: mainTokenTransferCapability
    },
    reconnect: state.auth.available ? state.auth.source === "legacy_viewer_auth_bootstrap" ? "reconnect still depends on the current preview bootstrap; hosted resume/revoke tokens are not wired yet" : state.auth.registrationStatus === "registered" ? "page reload will reuse the browser-local hosted key and attempt reconnect_sync first" : "browser-local hosted key is persisted, but runtime session restore is still pending this page load" : "page reload is possible, but player-session reconnect/resume is not implemented yet"
  };
}
function buildHostedActionMatrixView() {
  const matrix = Array.isArray(state.hostedAccess?.action_matrix) ? state.hostedAccess.action_matrix : [];
  return matrix.map((policy) => {
    const actionId = String(policy?.action_id || "").trim();
    const capability = buildSemanticCapability(actionId);
    return {
      actionId,
      requiredAuth: String(policy?.required_auth || "").trim() || "unknown",
      availability: String(policy?.availability || "").trim() || "unknown",
      reason: String(policy?.reason || capability.reason || "").trim(),
      enabled: capability.enabled === true,
      code: capability.code || null,
      capabilityReason: capability.reason || null
    };
  });
}
function buildHostedRecoveryHint(locale = state.uiLocale) {
  if (String(state.hostedAccess?.deployment_mode || "").trim() !== "hosted_public_join") {
    return null;
  }
  if (state.auth.available) {
    return null;
  }
  const errorText = String(state.auth.error || "").trim();
  const revokeReason = String(state.auth.revokeReason || "").trim();
  const revokedBy = String(state.auth.revokedBy || "").trim();
  if (!errorText) {
    return null;
  }
  if (errorText.includes("released locally")) {
    return {
      kind: "released",
      title: isLocaleZh(locale) ? "托管玩家会话已释放" : "Hosted player session released",
      detail: isLocaleZh(locale) ? "当前浏览器已经在本地释放托管玩家席位。若要继续试玩，需要重新获取托管玩家会话。" : "This browser returned its hosted player slot locally. Acquire a new hosted player session if you want to resume gameplay.",
      cta: isLocaleZh(locale) ? "获取托管玩家会话" : "Acquire Hosted Player Session"
    };
  }
  if (errorText.includes("revoked") || revokeReason || revokedBy) {
    const actorText = revokedBy ? ` by ${revokedBy}` : "";
    const reasonText = revokeReason ? ` Reason: ${revokeReason}.` : "";
    return {
      kind: "revoked",
      title: isLocaleZh(locale) ? "托管玩家会话已被撤销" : "Hosted player session was revoked",
      detail: isLocaleZh(locale) ? `运行时或操作者撤销了这个浏览器会话${actorText}.${reasonText} 继续进行玩法、聊天或 prompt 操作前，需要重新获取新的托管玩家会话。` : `The runtime or operator revoked this browser session${actorText}.${reasonText} You need to acquire a fresh hosted player session before gameplay, chat, or prompt actions can continue.`,
      cta: isLocaleZh(locale) ? "重新获取托管玩家会话" : "Re-acquire Hosted Player Session"
    };
  }
  if (errorText.includes("session_not_found") || errorText.includes("not found")) {
    return {
      kind: "missing",
      title: isLocaleZh(locale) ? "运行时中找不到托管玩家会话" : "Hosted player session is missing from runtime",
      detail: isLocaleZh(locale) ? "浏览器本地密钥仍存在，但运行时已经不再识别这个会话。请重新获取托管玩家会话并重新注册。" : "The browser-local key still exists, but the runtime no longer recognizes the session. Acquire a fresh hosted player session and register again.",
      cta: isLocaleZh(locale) ? "重新获取托管玩家会话" : "Re-acquire Hosted Player Session"
    };
  }
  return {
    kind: "guest",
    title: isLocaleZh(locale) ? "托管玩家会话不可用" : "Hosted player session is unavailable",
    detail: errorText,
    cta: isLocaleZh(locale) ? "获取托管玩家会话" : "Acquire Hosted Player Session"
  };
}
function nextRequestId() {
  requestId += 1;
  return requestId;
}
function nextAuthNonce() {
  authNonceCounter += 1;
  return Date.now() + authNonceCounter;
}
function snapshotControlFeedback(feedback) {
  if (!feedback) return null;
  return {
    id: feedback.id,
    action: feedback.action,
    accepted: feedback.accepted,
    stage: feedback.stage,
    reason: feedback.reason,
    hint: feedback.hint,
    effect: feedback.effect,
    deltaLogicalTime: feedback.deltaLogicalTime || 0,
    deltaEventSeq: feedback.deltaEventSeq || 0,
    deltaTraceCount: feedback.deltaTraceCount || 0
  };
}
function snapshotSemanticFeedback(feedback) {
  if (!feedback) return null;
  return {
    id: feedback.id,
    kind: feedback.kind,
    action: feedback.action,
    agentId: feedback.agentId || null,
    accepted: feedback.accepted,
    stage: feedback.stage,
    ok: feedback.ok,
    reason: feedback.reason || null,
    effect: feedback.effect || null,
    response: clone(feedback.response) || null
  };
}
function semanticFeedbackCode(feedback) {
  if (feedback?.stage !== "error") {
    return null;
  }
  const responseCode = String(feedback?.response?.code || "").trim();
  if (responseCode) {
    return responseCode;
  }
  const effectCode = String(feedback?.effect || "").trim();
  return effectCode || null;
}
function semanticFeedbackMessage(feedback) {
  const responseMessage = String(feedback?.response?.message || "").trim();
  if (responseMessage) {
    return responseMessage;
  }
  const reason = String(feedback?.reason || "").trim();
  return reason || null;
}
function formatPromptVersionLabel(value) {
  return `v${Math.max(0, Math.floor(Number(value || 0)))}`;
}
function humanizePromptField(field) {
  return String(field || "").trim().replaceAll("_", " ");
}
function summarizeAppliedFields(feedback) {
  const fields = Array.isArray(feedback?.response?.applied_fields) ? feedback.response.applied_fields.map(humanizePromptField).filter(Boolean) : [];
  if (!fields.length) {
    return null;
  }
  return fields.join(", ");
}
function describeSemanticFeedback(feedback, locale = state.uiLocale) {
  if (!feedback) {
    return null;
  }
  const code = semanticFeedbackCode(feedback);
  const diagnostics = semanticFeedbackMessage(feedback);
  const description = {
    label: feedback.stage || "idle",
    summary: feedback.effect || diagnostics || (isLocaleZh(locale) ? "反馈已更新。" : "Feedback updated."),
    detail: null,
    code,
    diagnostics,
    badgeClass: feedbackBadgeClass(feedback)
  };
  if (feedback.stage === "error") {
    if (code === "llm_init_failed") {
      description.label = isLocaleZh(locale) ? "LLM 不可用" : "LLM unavailable";
      description.summary = isLocaleZh(locale) ? "当前栈没有可用的 LLM 配置，因此无法开始聊天。" : "Chat cannot start because this stack has no usable LLM configuration.";
      description.detail = isLocaleZh(locale) ? "请把 model、base URL 和 API key 写入当前 config.toml 或 OASIS7_LLM_* 环境变量，然后重启 launcher 栈。" : "Add model, base URL, and API key to the active config.toml or OASIS7_LLM_* env, then restart the launcher stack.";
      return description;
    }
    if (code === "target_version_not_found") {
      description.label = isLocaleZh(locale) ? "找不到回滚目标" : "Rollback target missing";
      description.summary = isLocaleZh(locale) ? "当前 Agent 没有这个可回滚版本。" : "The selected rollback version is not available for this agent.";
      description.detail = isLocaleZh(locale) ? "请先刷新 prompt 状态，或改选一个真实存在的保存版本后再重试。" : "Refresh prompt state or choose an existing saved version before retrying.";
      return description;
    }
    if (code === "rollback_noop") {
      description.label = isLocaleZh(locale) ? "回滚无变化" : "Rollback noop";
      description.summary = isLocaleZh(locale) ? "这个回滚目标不会改变当前 prompt。" : "That rollback target would not change the current prompt.";
      description.detail = isLocaleZh(locale) ? "只有在你确实要恢复不同 prompt 内容时，才需要选择更旧的版本。" : "Pick an older version only when you need to restore different prompt content.";
      return description;
    }
    if (feedback.kind === "prompt") {
      description.label = isLocaleZh(locale) ? "Prompt 失败" : "Prompt failed";
      description.summary = isLocaleZh(locale) ? "Prompt 控制没有完成。" : "Prompt control did not complete.";
      description.detail = isLocaleZh(locale) ? "展开诊断可查看后端拒绝的具体原因。" : "Open diagnostics for the exact backend rejection.";
      return description;
    }
    if (feedback.kind === "chat") {
      description.label = isLocaleZh(locale) ? "聊天失败" : "Chat failed";
      description.summary = isLocaleZh(locale) ? "Agent 聊天没有完成。" : "Agent chat did not complete.";
      description.detail = isLocaleZh(locale) ? "展开诊断可查看后端拒绝的具体原因。" : "Open diagnostics for the exact backend rejection.";
      return description;
    }
    if (feedback.kind === "gameplay_action") {
      description.label = isLocaleZh(locale) ? "玩法动作失败" : "Gameplay action failed";
      description.summary = isLocaleZh(locale) ? "正式玩法动作没有完成。" : "The gameplay action did not complete.";
      description.detail = isLocaleZh(locale) ? "展开诊断可查看 runtime 返回的拒绝原因。" : "Open diagnostics for the runtime rejection details.";
      return description;
    }
    description.label = code || "Request failed";
    description.summary = diagnostics || (isLocaleZh(locale) ? "请求失败。" : "The request failed.");
    description.detail = isLocaleZh(locale) ? "展开诊断可查看后端原始载荷。" : "Open diagnostics for the raw backend payload.";
    return description;
  }
  if (feedback.kind === "prompt") {
    const version = Number(feedback?.response?.version || 0);
    const appliedFields = summarizeAppliedFields(feedback);
    if (feedback.stage === "preview_ack") {
      description.label = isLocaleZh(locale) ? "预览已就绪" : "Preview ready";
      description.summary = isLocaleZh(locale) ? `Prompt 预览已基于 ${formatPromptVersionLabel(version)} 准备完成。` : `Prompt preview is ready from ${formatPromptVersionLabel(version)}.`;
      description.detail = isLocaleZh(locale) ? "应用前请先检查返回的摘要或 prompt 字段。" : "Review the returned digest or prompt fields before applying.";
      return description;
    }
    if (feedback.stage === "apply_ack") {
      description.label = isLocaleZh(locale) ? "Prompt 已保存" : "Prompt saved";
      description.summary = isLocaleZh(locale) ? `Prompt 改动已保存为 ${formatPromptVersionLabel(version)}。` : `Prompt changes are now saved as ${formatPromptVersionLabel(version)}.`;
      description.detail = appliedFields ? isLocaleZh(locale) ? `已应用字段：${appliedFields}。` : `Applied fields: ${appliedFields}.` : isLocaleZh(locale) ? "Prompt 改动已被接受并持久化。" : "Prompt changes were accepted and persisted.";
      return description;
    }
    if (feedback.stage === "rollback_ack") {
      const restoredVersion = Number(feedback?.response?.rolled_back_to_version || 0);
      description.label = isLocaleZh(locale) ? "回滚已应用" : "Rollback applied";
      description.summary = isLocaleZh(locale) ? `当前生效 prompt 已保存为 ${formatPromptVersionLabel(version)}，其内容恢复自 ${formatPromptVersionLabel(restoredVersion)}。` : `Active prompt is now saved as ${formatPromptVersionLabel(version)} after restoring content from ${formatPromptVersionLabel(restoredVersion)}.`;
      description.detail = isLocaleZh(locale) ? "回滚会生成一个新的保存版本；下面输入框指向的是下一次回滚目标，不是刚刚恢复出来的版本。" : "Rollback creates a new saved version; the rollback input below points to the next target, not the version that was just restored.";
      return description;
    }
    description.label = isLocaleZh(locale) ? "Prompt 进行中" : "Prompt in progress";
    description.summary = feedback.effect || (isLocaleZh(locale) ? "Prompt 请求正在处理。" : "Prompt request is in flight.");
    description.detail = isLocaleZh(locale) ? "请等待 ack/error 返回后再发起下一次 prompt 操作。" : "Wait for ack/error before issuing another prompt action.";
    return description;
  }
  if (feedback.kind === "chat") {
    if (feedback.stage === "ack") {
      const acceptedAtTick = Number(feedback?.response?.accepted_at_tick || 0);
      description.label = isLocaleZh(locale) ? "聊天已受理" : "Chat accepted";
      description.summary = isLocaleZh(locale) ? `消息已在 tick ${acceptedAtTick} 进入 runtime 队列。` : `Message entered the runtime queue at tick ${acceptedAtTick}.`;
      description.detail = isLocaleZh(locale) ? "请查看 Message Flow，确认玩家出站消息和后续 Agent 回应。" : "Watch Message Flow for the outbound player message and any inbound agent reply.";
      return description;
    }
    description.label = isLocaleZh(locale) ? "聊天进行中" : "Chat in progress";
    description.summary = feedback.effect || (isLocaleZh(locale) ? "聊天请求正在处理。" : "Chat request is in flight.");
    description.detail = isLocaleZh(locale) ? "请等待 ack/error 返回后再发送下一条消息。" : "Wait for ack/error before sending another message.";
    return description;
  }
  if (feedback.kind === "gameplay_action") {
    if (feedback.stage === "ack") {
      const acceptedAtTick = Number(feedback?.response?.accepted_at_tick || 0);
      description.label = isLocaleZh(locale) ? "玩法动作已受理" : "Gameplay action accepted";
      description.summary = isLocaleZh(locale) ? `动作已在 tick ${acceptedAtTick} 进入 runtime 队列。` : `The action entered the runtime queue at tick ${acceptedAtTick}.`;
      description.detail = feedback?.response?.message || (isLocaleZh(locale) ? "请继续观察 gameplay feedback 或刷新后的快照。" : "Watch gameplay feedback or the refreshed snapshot for the next world-state change.");
      return description;
    }
    description.label = isLocaleZh(locale) ? "玩法动作进行中" : "Gameplay action in progress";
    description.summary = feedback.effect || (isLocaleZh(locale) ? "玩法动作请求正在处理。" : "Gameplay action request is in flight.");
    description.detail = isLocaleZh(locale) ? "请等待 ack/error 或新的 gameplay 快照反馈。" : "Wait for ack/error or a new gameplay snapshot update.";
    return description;
  }
  return description;
}
function describePromptVersionState(feedback = state.lastPromptFeedback, locale = state.uiLocale) {
  const currentVersion = Math.max(0, Math.floor(Number(state.promptDraft.currentVersion || 0)));
  const nextRollbackTargetVersion = Math.max(
    0,
    Math.floor(Number(state.promptDraft.rollbackTargetVersion || 0))
  );
  const responseVersion = Number(feedback?.response?.version);
  const ackVersion = Number.isFinite(responseVersion) ? Math.max(0, Math.floor(responseVersion)) : currentVersion;
  const responseRollbackVersion = Number(feedback?.response?.rolled_back_to_version);
  const restoredFromVersion = feedback?.stage === "rollback_ack" && Number.isFinite(responseRollbackVersion) ? Math.max(0, Math.floor(responseRollbackVersion)) : null;
  const summary = restoredFromVersion == null ? isLocaleZh(locale) ? `当前生效 prompt 版本是 ${formatPromptVersionLabel(currentVersion)}。` : `Active prompt version is ${formatPromptVersionLabel(currentVersion)}.` : isLocaleZh(locale) ? `当前生效 prompt 版本是 ${formatPromptVersionLabel(currentVersion)}；内容恢复自 ${formatPromptVersionLabel(restoredFromVersion)}。` : `Active prompt version is ${formatPromptVersionLabel(currentVersion)}; content was restored from ${formatPromptVersionLabel(restoredFromVersion)}.`;
  const detail = restoredFromVersion == null ? isLocaleZh(locale) ? `回滚输入框默认指向下一次目标 ${formatPromptVersionLabel(nextRollbackTargetVersion)}。` : `The rollback input defaults to the next target ${formatPromptVersionLabel(nextRollbackTargetVersion)}.` : isLocaleZh(locale) ? `这次回滚生成了新的保存版本 ${formatPromptVersionLabel(ackVersion)}。下面输入框现在指向下一次目标 ${formatPromptVersionLabel(nextRollbackTargetVersion)}，不是刚恢复的版本。` : `The rollback created a new saved version ${formatPromptVersionLabel(ackVersion)}. The input below now points to the next target ${formatPromptVersionLabel(nextRollbackTargetVersion)}, not the restored version.`;
  return {
    currentVersion,
    nextRollbackTargetVersion,
    ackVersion,
    restoredFromVersion,
    summary,
    detail
  };
}
function buildGameplaySummary(locale = state.uiLocale) {
  const gameplay = state.snapshot?.player_gameplay;
  if (!gameplay || typeof gameplay !== "object") {
    return null;
  }
  const agents = Object.keys(state.snapshot?.model?.agents || {});
  const locations = Object.keys(state.snapshot?.model?.locations || {});
  const missingAgents = agents.length === 0;
  const missingLocations = locations.length === 0;
  const emptyEntityBlocker = missingAgents || missingLocations ? (() => {
    const missing = [];
    if (missingAgents) {
      missing.push(isLocaleZh(locale) ? "Agent" : "agents");
    }
    if (missingLocations) {
      missing.push(isLocaleZh(locale) ? "地点" : "locations");
    }
    const missingLabel = missing.join(isLocaleZh(locale) ? " / " : "/");
    return {
      blockerKind: "runtime_snapshot_empty_entities",
      blockerDetail: isLocaleZh(locale) ? `runtime 已发布玩法进度，但当前快照没有 ${missingLabel}，formal web entry 暂时无法继续。` : `Runtime published gameplay progress, but the current snapshot has no ${missingLabel}; the formal web entry cannot continue yet.`,
      nextStepHint: isLocaleZh(locale) ? "先刷新快照；如果实体仍然为空，请修复或重启 runtime world bootstrap 后再继续。" : "Request a fresh snapshot first. If entities stay empty, repair or restart the runtime world bootstrap before continuing.",
      disabledReason: isLocaleZh(locale) ? `当前快照缺少 ${missingLabel}；刷新快照或修复 runtime bootstrap 后再试。` : `Current snapshot is missing ${missingLabel}; refresh the snapshot or repair runtime bootstrap before retrying.`
    };
  })() : null;
  const progressRaw = Number(gameplay.progress_percent);
  const progressPercent = Number.isFinite(progressRaw) ? Math.max(0, Math.min(100, Math.floor(progressRaw))) : null;
  const availableActions = Array.isArray(gameplay.available_actions) ? gameplay.available_actions.map((action) => ({
    actionId: action?.action_id || null,
    label: action?.label || null,
    protocolAction: action?.protocol_action || null,
    targetAgentId: action?.target_agent_id || null,
    disabledReason: action?.protocol_action === "request_snapshot" || action?.protocol_action === "world.request_snapshot" ? action?.disabled_reason || null : action?.disabled_reason || emptyEntityBlocker?.disabledReason || null,
    executeKind: action?.protocol_action === "request_snapshot" || action?.protocol_action === "world.request_snapshot" ? "request_snapshot" : action?.protocol_action === "live_control.step" ? "step" : action?.protocol_action === "live_control.play" ? "play" : action?.protocol_action === "gameplay_action.submit" ? "gameplay_action" : action?.protocol_action === "agent_chat" ? "agent_chat" : "unsupported"
  })) : [];
  const recentFeedback = gameplay.recent_feedback && typeof gameplay.recent_feedback === "object" ? {
    action: gameplay.recent_feedback.action || null,
    stage: gameplay.recent_feedback.stage || null,
    effect: gameplay.recent_feedback.effect || null,
    reason: gameplay.recent_feedback.reason || null,
    hint: gameplay.recent_feedback.hint || null,
    deltaLogicalTime: Number(gameplay.recent_feedback.delta_logical_time || 0),
    deltaEventSeq: Number(gameplay.recent_feedback.delta_event_seq || 0)
  } : null;
  const runtimeBlockerKind = gameplay.blocker_kind || null;
  const runtimeBlockerDetail = gameplay.blocker_detail || null;
  const runtimeAlreadyPublishedEmptyEntityBlocker = runtimeBlockerKind === "runtime_snapshot_empty_entities";
  return {
    stageId: gameplay.stage_id || null,
    stageStatus: emptyEntityBlocker ? "blocked" : gameplay.stage_status || null,
    goalId: gameplay.goal_id || null,
    goalKind: gameplay.goal_kind || null,
    goalTitle: gameplay.goal_title || null,
    objective: gameplay.objective || null,
    progressDetail: gameplay.progress_detail || null,
    progressPercent,
    blockerKind: runtimeAlreadyPublishedEmptyEntityBlocker ? runtimeBlockerKind : emptyEntityBlocker ? emptyEntityBlocker.blockerKind : runtimeBlockerKind,
    blockerDetail: runtimeAlreadyPublishedEmptyEntityBlocker ? runtimeBlockerDetail || emptyEntityBlocker?.blockerDetail || null : emptyEntityBlocker ? emptyEntityBlocker.blockerDetail : runtimeBlockerDetail,
    blockerSupplementalDetail: emptyEntityBlocker && runtimeBlockerDetail && !runtimeAlreadyPublishedEmptyEntityBlocker ? runtimeBlockerDetail : null,
    nextStepHint: runtimeAlreadyPublishedEmptyEntityBlocker ? gameplay.next_step_hint || emptyEntityBlocker?.nextStepHint || null : emptyEntityBlocker ? emptyEntityBlocker.nextStepHint : gameplay.next_step_hint || null,
    branchHint: gameplay.branch_hint || null,
    entityCounts: {
      agents: agents.length,
      locations: locations.length
    },
    availableActions,
    recentFeedback,
    agentClaim: clone(gameplay.agent_claim),
    assetGovernanceHandoff: isLocaleZh(locale) ? "资产 / 治理动作仍在单独 lane 处理；viewer 这里不会直接暴露主代币转账表单。" : "Asset/governance actions remain a separate lane. viewer exposes no main token transfer form here."
  };
}
function getState() {
  const authSurface = buildAuthSurfaceModel();
  const hostedActionMatrixView = buildHostedActionMatrixView();
  const hostedRecoveryHint = buildHostedRecoveryHint();
  const gameplaySummary = buildGameplaySummary();
  return {
    connectionStatus: state.connectionStatus,
    logicalTime: state.logicalTime,
    eventSeq: state.eventSeq,
    tick: state.tick,
    selectedKind: state.selectedKind,
    selectedId: state.selectedId,
    errorCount: state.errorCount,
    lastError: state.lastError,
    eventCount: state.eventCount,
    traceCount: state.traceCount,
    cameraMode: state.cameraMode,
    cameraRadius: state.cameraRadius,
    cameraOrthoScale: state.cameraOrthoScale,
    lastControlFeedback: snapshotControlFeedback(state.lastControlFeedback),
    lastPromptFeedback: snapshotSemanticFeedback(state.lastPromptFeedback),
    lastChatFeedback: snapshotSemanticFeedback(state.lastChatFeedback),
    lastGameplayActionFeedback: snapshotSemanticFeedback(state.lastGameplayActionFeedback),
    renderMode: state.renderMode,
    rendererClass: state.rendererClass,
    viewerReason: state.viewerReason,
    softwareSafeReason: state.viewerReason,
    renderer: state.renderer,
    vendor: state.vendor,
    webglVersion: state.webglVersion,
    pixelWorldRuntimeStatus: state.pixelWorldRuntimeStatus,
    pixelWorldRuntimeSource: state.pixelWorldRuntimeSource,
    pixelWorldRuntimeModuleUrl: state.pixelWorldRuntimeModuleUrl,
    pixelWorldCamera: clone(state.pixelWorldCamera),
    pixelWorldFatal: clone(state.pixelWorldFatal),
    uiLocale: state.uiLocale,
    promptOverridesVisible: state.promptOverridesVisible,
    controlProfile: state.controlProfile,
    debugViewerMode: state.debugViewerMode,
    debugViewerStatus: state.debugViewerStatus,
    worldId: state.worldId,
    server: state.server,
    wsUrl: state.wsUrl,
    authReady: state.auth.available,
    authPlayerId: state.auth.playerId,
    authPublicKey: state.auth.publicKey,
    authError: state.auth.error,
    authRevokeReason: state.auth.revokeReason,
    authRevokedBy: state.auth.revokedBy,
    authRegistrationStatus: state.auth.registrationStatus,
    authSessionEpoch: state.auth.sessionEpoch,
    authRecoveryErrorCode: state.auth.recoveryErrorCode,
    authRecoveryErrorMessage: state.auth.recoveryErrorMessage,
    authRuntimeStatus: state.auth.runtimeStatus,
    authBoundAgentId: state.auth.boundAgentId,
    authPendingRequestedAgentId: state.auth.pendingRequestedAgentId,
    authPendingForceRebind: state.auth.pendingForceRebind,
    authRebindNotice: state.auth.rebindNotice,
    authTier: authSurface.currentTier,
    authSource: authSurface.source,
    authDeploymentHint: authSurface.deploymentHint,
    authSurface: clone(authSurface),
    hostedRecoveryHint: clone(hostedRecoveryHint),
    hostedAccess: clone(state.hostedAccess),
    hostedActionMatrix: clone(hostedActionMatrixView),
    hostedAdmission: clone(state.hostedAdmission),
    gameplaySummary: clone(gameplaySummary),
    strongAuthApprovalCodeConfigured: !!String(state.strongAuth.approvalCode || "").trim(),
    strongAuthLastGrantActionId: state.strongAuth.lastGrantActionId,
    strongAuthLastGrantExpiresAtUnixMs: state.strongAuth.lastGrantExpiresAtUnixMs,
    strongAuthLastGrantError: state.strongAuth.lastGrantError,
    selectedAgentInteractionMode: selectedAgentInteractionMode(),
    selectedAgentDebug: clone(selectedAgentExecutionDebugContext()),
    selectedPromptVersion: state.promptDraft.currentVersion || 0,
    promptRollbackTargetVersion: state.promptDraft.rollbackTargetVersion || 0,
    chatHistoryCount: state.chatHistory.length,
    chatHistory: clone(state.chatHistory)
  };
}
function reportFatalError(message, source = "runtime") {
  const text = `${source}: ${String(message || "unknown runtime error")}`.trim();
  if (state.lastError !== text) {
    state.errorCount += 1;
  }
  state.connectionStatus = "error";
  state.debugViewerStatus = "error";
  state.lastError = text;
  render();
}
function parseSelectionPayload(payload) {
  if (payload == null) {
    return null;
  }
  if (typeof payload === "string") {
    const trimmed = payload.trim();
    if (!trimmed) return null;
    const parts = trimmed.split(":");
    if (parts.length >= 2) {
      return { kind: parts[0], id: parts.slice(1).join(":") };
    }
    return { kind: "agent", id: trimmed };
  }
  if (typeof payload === "object") {
    const kind = payload.kind || payload.targetKind || payload.type;
    const id = payload.id || payload.targetId || payload.value;
    if (!kind || !id) return null;
    return { kind: String(kind), id: String(id) };
  }
  return null;
}
function entityCollections() {
  const model = state.snapshot?.model || {};
  return {
    agents: Object.values(model.agents || {}),
    locations: Object.values(model.locations || {})
  };
}
function selectedAgentId() {
  return state.selectedKind === "agent" ? state.selectedId : null;
}
function selectedAgentPromptProfile() {
  const agentId = selectedAgentId();
  if (!agentId) {
    return null;
  }
  return state.snapshot?.model?.agent_prompt_profiles?.[agentId] || {
    agent_id: agentId,
    version: 0,
    updated_at_tick: 0,
    updated_by: "",
    system_prompt_override: null,
    short_term_goal_override: null,
    long_term_goal_override: null
  };
}
function selectedAgentBindingInfo() {
  const agentId = selectedAgentId();
  if (!agentId) {
    return null;
  }
  return {
    playerId: state.snapshot?.model?.agent_player_bindings?.[agentId] || null,
    publicKey: state.snapshot?.model?.agent_player_public_key_bindings?.[agentId] || null
  };
}
function selectedAgentExecutionDebugContext() {
  const agentId = selectedAgentId();
  if (!agentId) {
    return null;
  }
  return state.snapshot?.model?.agent_execution_debug_contexts?.[agentId] || null;
}
function selectedAgentInteractionMode() {
  const debugContext = selectedAgentExecutionDebugContext();
  if (debugContext?.provider_mode === "provider_loopback_http") {
    return "observer_only";
  }
  return "interactive";
}
function syncAgentInteractionDrafts(force = false) {
  const agentId = selectedAgentId();
  const profile = selectedAgentPromptProfile();
  if (force || state.promptDraft.agentId !== agentId || !state.promptDraft.dirty && agentId) {
    const currentVersion = Number(profile?.version || 0);
    state.promptDraft = {
      agentId,
      currentVersion,
      rollbackTargetVersion: Math.max(0, currentVersion - 1),
      updatedBy: String(profile?.updated_by || ""),
      updatedAtTick: Number(profile?.updated_at_tick || 0),
      systemPrompt: String(profile?.system_prompt_override || ""),
      shortTermGoal: String(profile?.short_term_goal_override || ""),
      longTermGoal: String(profile?.long_term_goal_override || ""),
      dirty: false
    };
  }
  if (force || state.chatDraft.agentId !== agentId) {
    state.chatDraft = {
      agentId,
      message: agentId === state.chatDraft.agentId ? state.chatDraft.message : "",
      dirty: false
    };
  }
}
function applySelection(selection) {
  if (!selection) return null;
  const kind = String(selection.kind || "").toLowerCase();
  const id = String(selection.id || "");
  const { agents, locations } = entityCollections();
  let object = null;
  if (kind === "agent") {
    object = agents.find((entry) => entry.id === id) || null;
  } else if (kind === "location") {
    object = locations.find((entry) => entry.id === id) || null;
  }
  if (!object) {
    return null;
  }
  state.selectedKind = kind;
  state.selectedId = id;
  state.selectedObject = object;
  syncAgentInteractionDrafts(true);
  render();
  return { kind, id };
}
function select(payload) {
  const parsed = parseSelectionPayload(payload);
  if (!parsed) {
    return { ok: false, reason: "invalid selection payload" };
  }
  const applied = applySelection(parsed);
  if (!applied) {
    return { ok: false, reason: `target not found: ${parsed.kind}:${parsed.id}` };
  }
  return { ok: true, ...applied };
}
function focus(payload) {
  return select(payload);
}
function parseStepCount(payload) {
  if (payload == null) return 1;
  if (typeof payload === "number" && Number.isFinite(payload) && payload >= 1) {
    return Math.floor(payload);
  }
  if (typeof payload === "string") {
    const trimmed = payload.trim();
    if (!trimmed || trimmed === "step") return 1;
    const numeric = Number(trimmed);
    if (Number.isFinite(numeric) && numeric >= 1) {
      return Math.floor(numeric);
    }
    const matched = trimmed.match(/step\s*[:=]\s*(\d+)/i);
    if (matched) {
      return Number(matched[1]);
    }
    return null;
  }
  if (typeof payload === "object") {
    const numeric = Number(payload.count);
    if (Number.isFinite(numeric) && numeric >= 1) {
      return Math.floor(numeric);
    }
  }
  return null;
}
function controlActions() {
  return [
    {
      action: "play",
      description: "Start continuous world advancement",
      descriptionZh: "开始连续推进世界",
      examplePayload: null
    },
    {
      action: "pause",
      description: "Pause continuous advancement",
      descriptionZh: "暂停连续推进",
      examplePayload: null
    },
    {
      action: "step",
      description: "Advance fixed steps (payload.count)",
      descriptionZh: "推进固定步数（payload.count）",
      examplePayload: { count: 5 }
    }
  ];
}
function describeControls() {
  return {
    controls: controlActions(),
    semanticActions: [
      {
        action: "sendAgentChat",
        description: "Send a player-authenticated chat message to an agent"
      },
      {
        action: "sendPromptControl",
        description: "Preview, apply, or rollback prompt overrides for an agent"
      }
    ],
    usage: "Use fillControlExample(action), sendControl(action), sendGameplayAction(actionIdOrPayload), sendAgentChat(agentId, message), sendPromptControl(mode, payload).",
    notes: [
      "viewer acts as a debug_viewer lane: it subscribes to runtime snapshots/events and does not own world authority",
      "when selectedAgentDebug.provider_mode=provider_loopback_http, prompt/chat stay observer-only in runtime live",
      "without viewer auth bootstrap the browser stays guest_session only; hosted public join player-session issuance is still pending"
    ]
  };
}
function fillControlExample(action) {
  const normalized = String(action || "").trim().toLowerCase();
  return controlActions().find((entry) => entry.action === normalized)?.examplePayload ?? null;
}
function sendJson(payload) {
  if (!socket || socket.readyState !== WebSocket.OPEN) {
    throw new Error("viewer websocket is not connected");
  }
  socket.send(JSON.stringify(payload));
}
function gameplayActionByProtocolAction(protocolAction) {
  const actions = state.snapshot?.player_gameplay?.available_actions;
  if (!Array.isArray(actions)) {
    return null;
  }
  return actions.find((action) => action?.protocol_action === protocolAction) || null;
}
function viewerControlGate(normalizedAction) {
  const protocolAction = state.controlProfile === "live" ? normalizedAction === "play" ? "live_control.play" : normalizedAction === "step" ? "live_control.step" : null : null;
  if (!protocolAction) {
    return null;
  }
  const gameplayAction = gameplayActionByProtocolAction(protocolAction);
  const disabledReason = String(gameplayAction?.disabled_reason || "").trim();
  if (!disabledReason) {
    return null;
  }
  return {
    reason: disabledReason,
    effect: `control blocked by gameplay gate: ${disabledReason}`,
    hint: state.snapshot?.player_gameplay?.next_step_hint || null
  };
}
function sendViewerControl(action, payload) {
  const normalized = String(action || "").trim().toLowerCase();
  const currentRequestId = nextRequestId();
  const feedback = {
    id: currentRequestId,
    action: normalized,
    accepted: false,
    stage: "rejected",
    reason: null,
    hint: null,
    effect: null,
    baselineLogicalTime: state.logicalTime,
    baselineEventSeq: state.eventSeq,
    deltaLogicalTime: 0,
    deltaEventSeq: 0,
    deltaTraceCount: 0,
    requestId: currentRequestId
  };
  let mode = null;
  if (normalized === "play") {
    mode = { mode: "play" };
  } else if (normalized === "pause") {
    mode = { mode: "pause" };
  } else if (normalized === "step") {
    const count = parseStepCount(payload);
    if (!count) {
      feedback.reason = "step requires numeric payload.count >= 1";
      feedback.effect = "request rejected before send";
      state.lastControlFeedback = feedback;
      render();
      return snapshotControlFeedback(feedback);
    }
    mode = { mode: "step", count };
  } else {
    feedback.reason = `unsupported action: ${normalized}`;
    feedback.effect = "request rejected before send";
    state.lastControlFeedback = feedback;
    render();
    return snapshotControlFeedback(feedback);
  }
  const gate = viewerControlGate(normalized);
  if (gate) {
    feedback.stage = "blocked";
    feedback.reason = gate.reason;
    feedback.hint = gate.hint;
    feedback.effect = gate.effect;
    state.lastControlFeedback = feedback;
    render();
    return snapshotControlFeedback(feedback);
  }
  try {
    if (state.controlProfile === "live") {
      sendJson({ type: "live_control", mode, request_id: currentRequestId });
    } else if (state.controlProfile === "playback") {
      sendJson({ type: "playback_control", mode, request_id: currentRequestId });
    } else {
      sendJson({ type: "control", mode, request_id: currentRequestId });
    }
    feedback.accepted = true;
    feedback.stage = "queued";
    feedback.effect = "queued, check getState().lastControlFeedback for world delta";
    pendingControlFeedback.set(currentRequestId, feedback);
    state.lastControlFeedback = feedback;
    render();
    return snapshotControlFeedback(feedback);
  } catch (error) {
    feedback.reason = String(error);
    feedback.effect = "request send failed";
    state.lastControlFeedback = feedback;
    render();
    return snapshotControlFeedback(feedback);
  }
}
function sendControl(action, payload = null) {
  return sendViewerControl(action, payload);
}
function runSteps(payload) {
  const count = parseStepCount(payload);
  if (!count) {
    return { ok: false, reason: "payload must be non-empty step string or count" };
  }
  const feedback = sendControl("step", { count });
  return { ok: Boolean(feedback?.accepted), count, feedback };
}
function setMode() {
  return {
    ok: false,
    reason: "viewer does not expose 2d/3d camera modes"
  };
}
function updateControlFeedbackFromProgress() {
  const feedback = state.lastControlFeedback;
  if (!feedback || !feedback.accepted) return;
  const deltaLogicalTime = Math.max(0, state.logicalTime - feedback.baselineLogicalTime);
  const deltaEventSeq = Math.max(0, state.eventSeq - feedback.baselineEventSeq);
  feedback.deltaLogicalTime = deltaLogicalTime;
  feedback.deltaEventSeq = deltaEventSeq;
  if (deltaLogicalTime > 0 || deltaEventSeq > 0) {
    feedback.stage = "completed_advanced";
    feedback.effect = `world advanced: logicalTime +${deltaLogicalTime}, eventSeq +${deltaEventSeq}`;
  }
}
function summarizeEventTitle(event) {
  const kind = event?.kind?.type || "unknown";
  return kind.replace(/_/g, " ");
}
function addRecentEvent(event) {
  state.recentEvents.unshift(event);
  state.recentEvents = state.recentEvents.slice(0, MAX_EVENTS);
  state.eventCount = state.recentEvents.length;
  state.eventSeq = Math.max(state.eventSeq, Number(event?.id || 0));
}
function handleSnapshot(snapshot) {
  state.snapshot = snapshot;
  state.logicalTime = Math.max(state.logicalTime, Number(snapshot?.time || 0));
  state.tick = state.logicalTime;
  const { agents, locations } = entityCollections();
  if (!state.selectedObject) {
    if (agents[0]) {
      applySelection({ kind: "agent", id: agents[0].id });
    } else if (locations[0]) {
      applySelection({ kind: "location", id: locations[0].id });
    }
  } else if (state.selectedKind && state.selectedId) {
    applySelection({ kind: state.selectedKind, id: state.selectedId });
  }
  syncAgentInteractionDrafts(false);
}
function injectSnapshot(snapshot) {
  if (!isTestApiEnabled()) {
    throw new Error("injectSnapshot requires test_api=1");
  }
  handleSnapshot(clone(snapshot));
  render();
  return getState();
}
function handleMetrics(time, metrics) {
  state.metrics = metrics || null;
  state.traceCount = Number(metrics?.decision_trace_count || 0);
  state.logicalTime = Math.max(state.logicalTime, Number(time || 0), Number(metrics?.total_ticks || 0));
  state.tick = state.logicalTime;
}
function handleControlCompletionAck(ack) {
  const feedback = pendingControlFeedback.get(ack?.request_id) || state.lastControlFeedback;
  if (!feedback) return;
  feedback.deltaLogicalTime = Number(ack?.delta_logical_time || 0);
  feedback.deltaEventSeq = Number(ack?.delta_event_seq || 0);
  if (ack?.status === "advanced") {
    feedback.stage = "completed_advanced";
    feedback.effect = `control ack advanced: logicalTime +${feedback.deltaLogicalTime}, eventSeq +${feedback.deltaEventSeq}`;
    feedback.reason = null;
  } else if (ack?.status === "blocked") {
    feedback.stage = "blocked";
    feedback.reason = ack?.error_message || ack?.error_code || "control was blocked before runtime advance";
    feedback.hint = state.snapshot?.player_gameplay?.next_step_hint || feedback.hint;
    feedback.effect = `gameplay blocked before requested advance completed: logicalTime +${feedback.deltaLogicalTime}, eventSeq +${feedback.deltaEventSeq}`;
  } else {
    feedback.stage = "completed_no_progress";
    feedback.reason = "timeout_no_progress";
    feedback.effect = `no visible world delta: logicalTime +${feedback.deltaLogicalTime}, eventSeq +${feedback.deltaEventSeq}`;
  }
  state.lastControlFeedback = feedback;
  pendingControlFeedback.delete(feedback.requestId);
}
function cborHeader(majorType, length) {
  if (!Number.isInteger(length) || length < 0) {
    throw new Error(`invalid CBOR length: ${length}`);
  }
  if (length < 24) {
    return Uint8Array.of(majorType << 5 | length);
  }
  if (length < 256) {
    return Uint8Array.of(majorType << 5 | 24, length);
  }
  if (length < 65536) {
    return Uint8Array.of(majorType << 5 | 25, length >> 8 & 255, length & 255);
  }
  if (length <= 4294967295) {
    return Uint8Array.of(
      majorType << 5 | 26,
      length >>> 24 & 255,
      length >>> 16 & 255,
      length >>> 8 & 255,
      length & 255
    );
  }
  if (length <= Number.MAX_SAFE_INTEGER) {
    const value = BigInt(length);
    return Uint8Array.of(
      majorType << 5 | 27,
      Number(value >> 56n & 0xffn),
      Number(value >> 48n & 0xffn),
      Number(value >> 40n & 0xffn),
      Number(value >> 32n & 0xffn),
      Number(value >> 24n & 0xffn),
      Number(value >> 16n & 0xffn),
      Number(value >> 8n & 0xffn),
      Number(value & 0xffn)
    );
  }
  throw new Error("CBOR length exceeds Number.MAX_SAFE_INTEGER");
}
function concatBytes(...parts) {
  const totalLength = parts.reduce((sum, bytes) => sum + bytes.length, 0);
  const out = new Uint8Array(totalLength);
  let offset = 0;
  for (const bytes of parts) {
    out.set(bytes, offset);
    offset += bytes.length;
  }
  return out;
}
function cborEncode(value) {
  if (value === null) {
    return Uint8Array.of(246);
  }
  if (value === false) {
    return Uint8Array.of(244);
  }
  if (value === true) {
    return Uint8Array.of(245);
  }
  if (typeof value === "number") {
    if (!Number.isInteger(value) || value < 0) {
      throw new Error(`unsupported CBOR number: ${value}`);
    }
    return cborHeader(0, value);
  }
  if (typeof value === "string") {
    const bytes = textEncoder.encode(value);
    return concatBytes(cborHeader(3, bytes.length), bytes);
  }
  if (Array.isArray(value)) {
    return concatBytes(cborHeader(4, value.length), ...value.map((entry) => cborEncode(entry)));
  }
  if (value instanceof Uint8Array) {
    return concatBytes(cborHeader(2, value.length), value);
  }
  if (typeof value === "object") {
    const entries = Object.entries(value).filter(([, entryValue]) => entryValue !== void 0);
    const encoded = [cborHeader(5, entries.length)];
    for (const [key, entryValue] of entries) {
      encoded.push(cborEncode(String(key)));
      encoded.push(cborEncode(entryValue));
    }
    return concatBytes(...encoded);
  }
  throw new Error(`unsupported CBOR type: ${typeof value}`);
}
function hexToBytes(raw) {
  const value = String(raw || "").trim().toLowerCase();
  if (!value || value.length % 2 !== 0 || /[^0-9a-f]/.test(value)) {
    throw new Error("invalid hex payload");
  }
  const bytes = new Uint8Array(value.length / 2);
  for (let index = 0; index < bytes.length; index += 1) {
    bytes[index] = Number.parseInt(value.slice(index * 2, index * 2 + 2), 16);
  }
  return bytes;
}
function bytesToHex(bytes) {
  return Array.from(bytes, (value) => value.toString(16).padStart(2, "0")).join("");
}
function bytesStartWith(bytes, prefix) {
  if (bytes.length < prefix.length) {
    return false;
  }
  for (let index = 0; index < prefix.length; index += 1) {
    if (bytes[index] !== prefix[index]) {
      return false;
    }
  }
  return true;
}
async function importEd25519SigningKey(privateKeyHex) {
  if (!window.crypto?.subtle) {
    throw new Error("Web Crypto subtle API is unavailable");
  }
  if (!authKeyCache.has(privateKeyHex)) {
    const rawPrivateKey = hexToBytes(privateKeyHex);
    if (rawPrivateKey.length !== 32) {
      throw new Error(`viewer auth private key length mismatch: expected 32 bytes, got ${rawPrivateKey.length}`);
    }
    const pkcs8 = concatBytes(ED25519_PKCS8_PREFIX, rawPrivateKey);
    authKeyCache.set(
      privateKeyHex,
      window.crypto.subtle.importKey("pkcs8", pkcs8, { name: "Ed25519" }, false, ["sign"])
    );
  }
  return authKeyCache.get(privateKeyHex);
}
async function signAuthPayload(signingPayloadBytes, auth) {
  const key = await importEd25519SigningKey(auth.privateKey);
  const signature = await window.crypto.subtle.sign({ name: "Ed25519" }, key, signingPayloadBytes);
  return `${VIEWER_AUTH_SIGNATURE_PREFIX}${bytesToHex(new Uint8Array(signature))}`;
}
async function generateEphemeralEd25519Keypair() {
  if (!window.crypto?.subtle) {
    throw new Error("Web Crypto subtle API is unavailable");
  }
  const keyPair = await window.crypto.subtle.generateKey(
    { name: "Ed25519" },
    true,
    ["sign", "verify"]
  );
  const pkcs8 = new Uint8Array(await window.crypto.subtle.exportKey("pkcs8", keyPair.privateKey));
  if (!bytesStartWith(pkcs8, ED25519_PKCS8_PREFIX) || pkcs8.length !== ED25519_PKCS8_PREFIX.length + 32) {
    throw new Error("unexpected Ed25519 pkcs8 encoding from Web Crypto");
  }
  const rawPublicKey = new Uint8Array(await window.crypto.subtle.exportKey("raw", keyPair.publicKey));
  if (rawPublicKey.length !== 32) {
    throw new Error(`unexpected Ed25519 public key length: ${rawPublicKey.length}`);
  }
  return {
    publicKey: bytesToHex(rawPublicKey),
    privateKey: bytesToHex(pkcs8.slice(ED25519_PKCS8_PREFIX.length))
  };
}
function buildAuthEnvelope(payload) {
  return cborEncode({
    version: 1,
    payload
  });
}
async function buildAgentChatAuthProof(request, auth) {
  const nonce = nextAuthNonce();
  const payload = {
    operation: "agent_chat",
    agent_id: request.agent_id,
    player_id: auth.playerId,
    public_key: auth.publicKey,
    nonce,
    message: request.message
  };
  if (request.intent_tick != null) {
    payload.intent_tick = request.intent_tick;
  }
  if (request.intent_seq != null) {
    payload.intent_seq = request.intent_seq;
  }
  const signingPayload = buildAuthEnvelope(payload);
  return {
    scheme: "ed25519",
    player_id: auth.playerId,
    public_key: auth.publicKey,
    nonce,
    signature: await signAuthPayload(signingPayload, auth)
  };
}
function promptPatchFromDraft(currentValue, draftValue) {
  const current = currentValue == null ? "" : String(currentValue);
  const draft = String(draftValue ?? "");
  if (draft === current) {
    return { mode: "unchanged" };
  }
  if (draft.length === 0) {
    return currentValue == null ? { mode: "unchanged" } : { mode: "clear" };
  }
  return { mode: "set", value: draft };
}
async function buildPromptControlAuthProof(mode, request, auth) {
  const nonce = nextAuthNonce();
  const payload = {
    operation: mode === "preview" ? "prompt_control_preview" : "prompt_control_apply",
    agent_id: request.agent_id,
    player_id: auth.playerId,
    public_key: auth.publicKey,
    nonce,
    expected_version: request.expected_version ?? null,
    updated_by: request.updated_by ?? null,
    system_prompt_override: request.system_prompt_override,
    short_term_goal_override: request.short_term_goal_override,
    long_term_goal_override: request.long_term_goal_override
  };
  const signingPayload = buildAuthEnvelope(payload);
  return {
    scheme: "ed25519",
    player_id: auth.playerId,
    public_key: auth.publicKey,
    nonce,
    signature: await signAuthPayload(signingPayload, auth)
  };
}
async function buildPromptRollbackAuthProof(request, auth) {
  const nonce = nextAuthNonce();
  const payload = {
    operation: "prompt_control_rollback",
    agent_id: request.agent_id,
    player_id: auth.playerId,
    public_key: auth.publicKey,
    nonce,
    to_version: request.to_version,
    expected_version: request.expected_version ?? null,
    updated_by: request.updated_by ?? null
  };
  const signingPayload = buildAuthEnvelope(payload);
  return {
    scheme: "ed25519",
    player_id: auth.playerId,
    public_key: auth.publicKey,
    nonce,
    signature: await signAuthPayload(signingPayload, auth)
  };
}
async function buildSessionRegisterAuthProof(request, auth) {
  const nonce = nextAuthNonce();
  const payload = {
    operation: "session_register",
    player_id: auth.playerId,
    public_key: auth.publicKey,
    nonce
  };
  if (request.requested_agent_id != null) {
    payload.requested_agent_id = request.requested_agent_id;
  }
  payload.force_rebind = request.force_rebind === true;
  const signingPayload = buildAuthEnvelope(payload);
  return {
    scheme: "ed25519",
    player_id: auth.playerId,
    public_key: auth.publicKey,
    nonce,
    signature: await signAuthPayload(signingPayload, auth)
  };
}
async function buildGameplayActionAuthProof(request, auth) {
  const nonce = nextAuthNonce();
  const payload = {
    operation: "gameplay_action",
    action_id: request.action_id,
    target_agent_id: request.target_agent_id,
    player_id: auth.playerId,
    public_key: auth.publicKey,
    nonce
  };
  if (request.actor_agent_id != null) {
    payload.actor_agent_id = request.actor_agent_id;
  }
  const signingPayload = buildAuthEnvelope(payload);
  return {
    scheme: "ed25519",
    player_id: auth.playerId,
    public_key: auth.publicKey,
    nonce,
    signature: await signAuthPayload(signingPayload, auth)
  };
}
function canAutoIssueHostedPlayerSession() {
  return String(state.hostedAccess?.deployment_mode || "").trim() === "hosted_public_join" && state.auth.source !== "legacy_viewer_auth_bootstrap";
}
async function issueHostedPlayerIdentity() {
  if (!canAutoIssueHostedPlayerSession()) {
    return state.auth;
  }
  if (state.auth.available || state.auth.issueInFlight) {
    return state.auth;
  }
  state.auth.issueInFlight = true;
  state.auth.error = null;
  render();
  try {
    const response = await fetch(HOSTED_PLAYER_SESSION_ISSUE_ROUTE, {
      method: "GET",
      cache: "no-store",
      headers: { Accept: "application/json" }
    });
    const payload = await response.json();
    if (!response.ok || !payload?.ok || !payload?.grant?.player_id) {
      if (payload?.admission) {
        state.hostedAdmission = clone(payload.admission);
      }
      throw new Error(payload?.error || payload?.error_code || `hosted player-session issue failed with HTTP ${response.status}`);
    }
    state.hostedAdmission = payload?.admission ? clone(payload.admission) : state.hostedAdmission;
    const keypair = await generateEphemeralEd25519Keypair();
    state.auth = {
      available: true,
      playerId: String(payload.grant.player_id || "").trim(),
      publicKey: keypair.publicKey,
      privateKey: keypair.privateKey,
      releaseToken: String(payload.grant.release_token || "").trim() || null,
      error: null,
      revokeReason: null,
      revokedBy: null,
      source: "hosted_browser_storage",
      registrationStatus: "issued",
      sessionEpoch: null,
      issuedAtUnixMs: payload?.grant?.issued_at_unix_ms == null ? Date.now() : Number(payload.grant.issued_at_unix_ms),
      recoveryErrorCode: null,
      recoveryErrorMessage: null,
      issueInFlight: false,
      syncInFlight: false,
      runtimeStatus: "issued",
      boundAgentId: null,
      pendingRequestedAgentId: null,
      pendingForceRebind: false,
      rebindNotice: null
    };
    persistHostedPlayerSession(state.auth);
    render();
    return state.auth;
  } catch (error) {
    state.auth.issueInFlight = false;
    state.auth.error = String(error);
    render();
    return state.auth;
  }
}
async function ensureHostedPlayerAuthAvailable() {
  if (state.auth.available) {
    return state.auth;
  }
  if (canAutoIssueHostedPlayerSession()) {
    return issueHostedPlayerIdentity();
  }
  return state.auth;
}
async function retryHostedPlayerIdentityIssue() {
  if (!canAutoIssueHostedPlayerSession()) {
    return { ok: false, reason: "hosted public player-session issue is unavailable on this lane" };
  }
  const auth = await issueHostedPlayerIdentity();
  render();
  return {
    ok: auth.available,
    playerId: auth.playerId,
    error: auth.error
  };
}
async function requestHostedStrongAuthGrant(actionId, agentId) {
  const playerId = String(state.auth.playerId || "").trim();
  const publicKey = String(state.auth.publicKey || "").trim();
  const releaseToken = String(state.auth.releaseToken || "").trim();
  const approvalCode = String(state.strongAuth.approvalCode || "").trim();
  if (!playerId || !publicKey || !releaseToken) {
    throw new Error("hosted strong-auth grant requires an active player_session with release token");
  }
  if (!approvalCode) {
    throw new Error("backend approval code is required before hosted strong auth can be granted");
  }
  const query = new URLSearchParams({
    player_id: playerId,
    public_key: publicKey,
    release_token: releaseToken,
    agent_id: String(agentId || "").trim(),
    action_id: String(actionId || "").trim(),
    approval_code: approvalCode
  });
  const response = await fetch(`${HOSTED_STRONG_AUTH_GRANT_ROUTE}?${query.toString()}`, {
    method: "GET",
    cache: "no-store",
    headers: { Accept: "application/json" }
  });
  const payload = await response.json();
  if (payload?.admission) {
    state.hostedAdmission = clone(payload.admission);
  }
  if (!response.ok || !payload?.ok || !payload?.grant) {
    state.strongAuth.lastGrantError = payload?.error || payload?.error_code || `hosted strong-auth grant failed with HTTP ${response.status}`;
    throw new Error(state.strongAuth.lastGrantError);
  }
  state.strongAuth.lastGrantActionId = String(payload.grant.action_id || "").trim() || actionId;
  state.strongAuth.lastGrantExpiresAtUnixMs = payload?.grant?.expires_at_unix_ms == null ? null : Number(payload.grant.expires_at_unix_ms);
  state.strongAuth.lastGrantError = null;
  return payload.grant;
}
function sendReconnectSync() {
  if (!state.auth.available || state.auth.source === "legacy_viewer_auth_bootstrap") {
    return;
  }
  state.auth.syncInFlight = true;
  state.auth.registrationStatus = "registering";
  state.auth.runtimeStatus = "probing";
  state.auth.recoveryErrorCode = null;
  state.auth.recoveryErrorMessage = null;
  sendJson({
    type: "authoritative_recovery",
    command: {
      mode: "reconnect_sync",
      request: {
        player_id: state.auth.playerId,
        session_pubkey: state.auth.publicKey
      }
    }
  });
}
function probeHostedRuntimeSession() {
  if (!state.auth.available || state.auth.source === "legacy_viewer_auth_bootstrap" || state.connectionStatus !== "connected" || state.auth.registrationStatus !== "registered") {
    return;
  }
  state.auth.syncInFlight = true;
  state.auth.runtimeStatus = "probing";
  sendJson({
    type: "authoritative_recovery",
    command: {
      mode: "reconnect_sync",
      request: {
        player_id: state.auth.playerId,
        session_pubkey: state.auth.publicKey
      }
    }
  });
}
async function releaseHostedPlayerSlot() {
  const playerId = String(state.auth.playerId || "").trim();
  const releaseToken = String(state.auth.releaseToken || "").trim();
  if (!playerId || !releaseToken || state.auth.source === "legacy_viewer_auth_bootstrap") {
    return { ok: false, skipped: true };
  }
  const query = `player_id=${encodeURIComponent(playerId)}&release_token=${encodeURIComponent(releaseToken)}`;
  const response = await fetch(`${HOSTED_PLAYER_SESSION_RELEASE_ROUTE}?${query}`, {
    method: "POST",
    cache: "no-store",
    headers: { Accept: "application/json" }
  });
  const payload = await response.json();
  if (!response.ok || !payload?.ok) {
    if (payload?.admission) {
      state.hostedAdmission = clone(payload.admission);
    }
    throw new Error(payload?.error || payload?.error_code || `hosted player-session release failed with HTTP ${response.status}`);
  }
  state.hostedAdmission = payload?.admission ? clone(payload.admission) : state.hostedAdmission;
  return payload;
}
function resetHostedPlayerAuthState(errorMessage = null, revocationMeta = null) {
  stopHostedSessionRefreshLoop();
  clearHostedPlayerSession();
  const bootstrap2 = resolveAuthBootstrap();
  const revokeReason = String(revocationMeta?.revokeReason || "").trim() || null;
  const revokedBy = String(revocationMeta?.revokedBy || "").trim() || null;
  state.auth = bootstrap2.available ? bootstrap2 : {
    ...bootstrap2,
    source: "guest_only",
    registrationStatus: "guest",
    error: errorMessage,
    revokeReason,
    revokedBy,
    sessionEpoch: null,
    issuedAtUnixMs: null,
    releaseToken: null,
    recoveryErrorCode: null,
    recoveryErrorMessage: null,
    issueInFlight: false,
    syncInFlight: false,
    runtimeStatus: "guest",
    boundAgentId: null,
    pendingRequestedAgentId: null,
    pendingForceRebind: false,
    rebindNotice: null
  };
  void refreshHostedAdmissionState().then(() => render());
}
async function logoutHostedPlayerSession() {
  if (!state.auth.available || state.auth.source === "legacy_viewer_auth_bootstrap") {
    return { ok: false, reason: "hosted browser session is unavailable" };
  }
  const revokeRequest = {
    player_id: state.auth.playerId,
    session_pubkey: state.auth.publicKey,
    revoke_reason: "player_logout",
    revoked_by: state.auth.playerId
  };
  try {
    if (state.connectionStatus === "connected") {
      sendJson({
        type: "authoritative_recovery",
        command: {
          mode: "revoke_session",
          request: revokeRequest
        }
      });
    }
  } catch (_) {
  }
  try {
    await releaseHostedPlayerSlot();
  } finally {
    resetHostedPlayerAuthState("hosted player session released locally");
    render();
  }
  return { ok: true };
}
function syncHostedPlayerSessionOnConnect() {
  if (!state.auth.available || state.auth.source === "legacy_viewer_auth_bootstrap" || state.auth.syncInFlight) {
    return;
  }
  sendReconnectSync();
}
function clearPendingSessionRegisterWaiter(error = null) {
  if (!pendingSessionRegisterWaiter) {
    return;
  }
  const waiter = pendingSessionRegisterWaiter;
  pendingSessionRegisterWaiter = null;
  if (error != null) {
    waiter.reject(error instanceof Error ? error : new Error(String(error)));
  }
}
async function dispatchSessionRegisterRequest(requestedAgentId, forceRebind) {
  const normalizedRequestedAgentId = String(requestedAgentId || "").trim() || null;
  if (state.auth.source !== "legacy_viewer_auth_bootstrap") {
    state.auth.registrationStatus = "registering";
    state.auth.syncInFlight = true;
    state.auth.recoveryErrorCode = null;
    state.auth.recoveryErrorMessage = null;
    state.auth.runtimeStatus = forceRebind === true ? "rebind_registering" : "registering";
  }
  if (forceRebind === true) {
    state.auth.rebindNotice = `Switching player session to ${normalizedRequestedAgentId || "requested agent"}...`;
  }
  state.auth.pendingRequestedAgentId = normalizedRequestedAgentId;
  state.auth.pendingForceRebind = forceRebind === true;
  const request = {
    player_id: state.auth.playerId,
    public_key: state.auth.publicKey
  };
  if (normalizedRequestedAgentId) {
    request.requested_agent_id = normalizedRequestedAgentId;
  }
  if (forceRebind === true) {
    request.force_rebind = true;
  }
  request.auth = await buildSessionRegisterAuthProof(request, state.auth);
  sendJson({
    type: "authoritative_recovery",
    command: {
      mode: "register_session",
      request
    }
  });
  render();
}
async function retryPendingSessionRegisterWaiterWithForceRebind() {
  const waiter = pendingSessionRegisterWaiter;
  if (!waiter) {
    return;
  }
  waiter.forceRebind = true;
  try {
    await dispatchSessionRegisterRequest(waiter.requestedAgentId, true);
  } catch (error) {
    clearPendingSessionRegisterWaiter(error);
    throw error;
  }
}
function latestRequestedAgentId(fallbackAgentId = null) {
  const agentId = String(
    fallbackAgentId || state.auth.pendingRequestedAgentId || state.auth.boundAgentId || ""
  ).trim();
  return agentId || null;
}
function recoveryErrorRequiresExplicitRebind(error) {
  return String(error?.code || "").trim() === "player_bind_failed" && String(error?.message || "").includes("explicit rebind required");
}
async function ensureRegisteredPlayerSession(requestedAgentId = null, options = {}) {
  await ensureHostedPlayerAuthAvailable();
  if (!state.auth.available) {
    throw new Error(state.auth.error || "player session auth is unavailable");
  }
  const normalizedRequestedAgentId = String(requestedAgentId || "").trim() || null;
  const forceRebind = options?.forceRebind === true;
  if (state.auth.registrationStatus === "registered" && (state.auth.runtimeStatus === "registered" || state.auth.runtimeStatus === "registered_unbound") && !forceRebind && (normalizedRequestedAgentId == null || normalizedRequestedAgentId === state.auth.boundAgentId)) {
    return state.auth;
  }
  if (pendingSessionRegisterWaiter) {
    const sameRequest = pendingSessionRegisterWaiter.requestedAgentId === normalizedRequestedAgentId && pendingSessionRegisterWaiter.forceRebind === forceRebind;
    if (!sameRequest) {
      throw new Error("another player session registration is already in flight");
    }
    return pendingSessionRegisterWaiter.promise;
  }
  let resolveWaiter;
  let rejectWaiter;
  const promise = new Promise((resolve, reject) => {
    resolveWaiter = resolve;
    rejectWaiter = reject;
  });
  pendingSessionRegisterWaiter = {
    requestedAgentId: normalizedRequestedAgentId,
    forceRebind,
    promise,
    resolve: resolveWaiter,
    reject: rejectWaiter
  };
  try {
    await dispatchSessionRegisterRequest(normalizedRequestedAgentId, forceRebind);
  } catch (error) {
    clearPendingSessionRegisterWaiter(error);
    throw error;
  }
  return promise;
}
function buildPromptRequestFromDraft(agentId, draftOverrides) {
  const currentProfile = selectedAgentPromptProfile();
  if (!agentId || !currentProfile) {
    throw new Error("select an agent before editing prompt overrides");
  }
  return {
    agent_id: agentId,
    player_id: state.auth.playerId,
    public_key: state.auth.publicKey,
    expected_version: Number(currentProfile.version || 0),
    updated_by: state.auth.playerId,
    system_prompt_override: promptPatchFromDraft(currentProfile.system_prompt_override, draftOverrides.systemPrompt),
    short_term_goal_override: promptPatchFromDraft(currentProfile.short_term_goal_override, draftOverrides.shortTermGoal),
    long_term_goal_override: promptPatchFromDraft(currentProfile.long_term_goal_override, draftOverrides.longTermGoal)
  };
}
function encodePromptRequestForJson(request) {
  const encodePatch = (patch) => {
    if (!patch || patch.mode === "unchanged") {
      return void 0;
    }
    if (patch.mode === "clear") {
      return null;
    }
    return patch.value;
  };
  return {
    agent_id: request.agent_id,
    player_id: request.player_id,
    public_key: request.public_key,
    expected_version: request.expected_version,
    updated_by: request.updated_by,
    system_prompt_override: encodePatch(request.system_prompt_override),
    short_term_goal_override: encodePatch(request.short_term_goal_override),
    long_term_goal_override: encodePatch(request.long_term_goal_override)
  };
}
function buildPromptRollbackRequest(agentId, toVersion) {
  const profile = selectedAgentPromptProfile();
  const targetVersion = Number(toVersion);
  if (!agentId || !profile) {
    throw new Error("select an agent before rolling back prompt overrides");
  }
  if (!Number.isInteger(targetVersion) || targetVersion < 0) {
    throw new Error("prompt rollback requires integer toVersion >= 0");
  }
  return {
    agent_id: agentId,
    player_id: state.auth.playerId,
    public_key: state.auth.publicKey,
    to_version: targetVersion,
    expected_version: Number(profile.version || 0),
    updated_by: state.auth.playerId
  };
}
function pushChatHistory(entry) {
  if (!entry) {
    return;
  }
  state.chatHistory.unshift({
    id: entry.id || `${entry.source || "chat"}-${Date.now()}-${Math.random().toString(16).slice(2)}`,
    source: entry.source || "event",
    agentId: entry.agentId || null,
    locationId: entry.locationId || null,
    message: String(entry.message || ""),
    tick: Number(entry.tick || 0),
    speaker: entry.speaker || null,
    playerId: entry.playerId || null,
    targetAgentId: entry.targetAgentId || null,
    intentSeq: entry.intentSeq || null
  });
  state.chatHistory = state.chatHistory.slice(0, 40);
}
function extractAgentSpokeEntry(event) {
  const kind = event?.kind;
  const kindType = String(kind?.type || "");
  if (!["agent_spoke", "AgentSpoke"].includes(kindType)) {
    return null;
  }
  const data = kind.data || {};
  return {
    id: `event-${event.id}`,
    source: "event",
    agentId: data.agent_id || null,
    locationId: data.location_id || null,
    message: data.message || "",
    tick: Number(event.time || 0),
    speaker: data.agent_id || null,
    targetAgentId: data.target_agent_id || null
  };
}
function requestSnapshotSafe() {
  try {
    sendJson({ type: "request_snapshot" });
  } catch (_) {
  }
}
function createSemanticFeedback(kind, action, agentId, extra = {}) {
  return {
    id: nextRequestId(),
    kind,
    action,
    agentId,
    accepted: true,
    ok: false,
    stage: "queued",
    reason: null,
    effect: null,
    response: null,
    ...extra
  };
}
function markPendingSemanticRebind(message) {
  const text = String(message).trim();
  for (const feedback of [state.lastChatFeedback, state.lastPromptFeedback]) {
    if (!feedback || feedback.stage !== "registering") {
      continue;
    }
    feedback.effect = text;
    feedback.reason = null;
  }
}
function enqueueSemanticCommand(command) {
  pendingSemanticCommands.push(command);
  if (!semanticSendLoop) {
    semanticSendLoop = processSemanticCommands();
  }
}
async function processSemanticCommands() {
  try {
    while (pendingSemanticCommands.length > 0) {
      const command = pendingSemanticCommands.shift();
      try {
        await command.execute();
      } catch (error) {
        command.feedback.stage = "error";
        command.feedback.ok = false;
        command.feedback.reason = String(error);
        command.feedback.effect = "request build/send failed";
        if (command.kind === "chat") {
          state.lastChatFeedback = command.feedback;
        } else {
          state.lastPromptFeedback = command.feedback;
        }
        render();
      }
    }
  } finally {
    semanticSendLoop = null;
    if (pendingSemanticCommands.length > 0) {
      semanticSendLoop = processSemanticCommands();
    }
  }
}
function assertSemanticCapability(actionId) {
  const capability = buildSemanticCapability(actionId);
  if (!capability.enabled) {
    throw new Error(capability.reason || state.auth.error || `${actionId} is unavailable`);
  }
}
function sendAgentChat(agentIdOrPayload, maybeMessage) {
  let agentId = null;
  let message = null;
  if (typeof agentIdOrPayload === "object" && agentIdOrPayload !== null) {
    agentId = String(agentIdOrPayload.agentId || agentIdOrPayload.agent_id || selectedAgentId() || "");
    message = String(agentIdOrPayload.message || "");
  } else {
    agentId = String(agentIdOrPayload || selectedAgentId() || "");
    message = String(maybeMessage || "");
  }
  if (!agentId) {
    return { ok: false, reason: "agent chat requires a selected agent or explicit agentId" };
  }
  if (!message.trim()) {
    return { ok: false, reason: "agent chat message cannot be empty" };
  }
  const feedback = createSemanticFeedback("chat", "agent_chat", agentId, {
    effect: "queued for signing and send",
    pendingMessage: message,
    pendingPlayerId: state.auth.playerId || null
  });
  state.lastChatFeedback = feedback;
  enqueueSemanticCommand({
    kind: "chat",
    feedback,
    execute: async () => {
      await ensureHostedPlayerAuthAvailable();
      assertSemanticCapability("agent_chat");
      feedback.stage = "registering";
      feedback.effect = "registering player session";
      render();
      await ensureRegisteredPlayerSession(agentId);
      feedback.stage = "signing";
      feedback.effect = "building auth proof";
      render();
      const request = {
        agent_id: agentId,
        message,
        player_id: state.auth.playerId,
        public_key: state.auth.publicKey
      };
      request.auth = await buildAgentChatAuthProof(request, state.auth);
      feedback.stage = "sent";
      feedback.effect = "agent_chat request sent; waiting for ack";
      state.lastChatFeedback = feedback;
      sendJson({ type: "agent_chat", request });
      state.chatDraft.message = "";
      state.chatDraft.dirty = false;
      render();
    }
  });
  render();
  return { ok: true, feedback: snapshotSemanticFeedback(feedback) };
}
function sendPromptControl(mode, payload = null) {
  const normalizedMode = String(mode || "").trim().toLowerCase();
  if (!["preview", "apply", "rollback"].includes(normalizedMode)) {
    return { ok: false, reason: "prompt control mode must be preview, apply, or rollback" };
  }
  const selectedId = selectedAgentId();
  const agentId = String(payload?.agentId || payload?.agent_id || selectedId || "");
  if (!agentId) {
    return { ok: false, reason: "prompt control requires a selected agent or explicit agentId" };
  }
  let request;
  try {
    if (normalizedMode === "rollback") {
      const currentVersion = Number(state.promptDraft.currentVersion || selectedAgentPromptProfile()?.version || 0);
      const fallbackVersion = Math.max(0, currentVersion - 1);
      const toVersion = payload?.toVersion ?? payload?.to_version ?? fallbackVersion;
      request = buildPromptRollbackRequest(agentId, toVersion);
    } else {
      request = buildPromptRequestFromDraft(agentId, {
        systemPrompt: payload?.systemPrompt ?? payload?.system_prompt_override ?? state.promptDraft.systemPrompt,
        shortTermGoal: payload?.shortTermGoal ?? payload?.short_term_goal_override ?? state.promptDraft.shortTermGoal,
        longTermGoal: payload?.longTermGoal ?? payload?.long_term_goal_override ?? state.promptDraft.longTermGoal
      });
    }
  } catch (error) {
    return { ok: false, reason: String(error) };
  }
  const feedback = createSemanticFeedback("prompt", `prompt_${normalizedMode}`, agentId, {
    effect: "queued for signing and send",
    toVersion: request.to_version ?? null
  });
  state.lastPromptFeedback = feedback;
  enqueueSemanticCommand({
    kind: "prompt",
    feedback,
    execute: async () => {
      await ensureHostedPlayerAuthAvailable();
      assertSemanticCapability("prompt_control");
      feedback.stage = "registering";
      feedback.effect = "registering player session";
      render();
      await ensureRegisteredPlayerSession(agentId);
      let strongAuthGrant = null;
      if (String(state.hostedAccess?.deployment_mode || "").trim() === "hosted_public_join") {
        feedback.stage = "authorizing";
        feedback.effect = "requesting backend strong-auth grant";
        render();
        strongAuthGrant = await requestHostedStrongAuthGrant(
          normalizedMode === "rollback" ? "prompt_control_rollback" : `prompt_control_${normalizedMode}`,
          agentId
        );
      }
      feedback.stage = "signing";
      feedback.effect = "building auth proof";
      render();
      let commandRequest;
      if (normalizedMode === "rollback") {
        commandRequest = {
          ...request,
          auth: await buildPromptRollbackAuthProof(request, state.auth)
        };
        if (strongAuthGrant) {
          commandRequest.strong_auth_grant = strongAuthGrant;
        }
      } else {
        commandRequest = encodePromptRequestForJson(request);
        commandRequest.auth = await buildPromptControlAuthProof(normalizedMode, request, state.auth);
        if (strongAuthGrant) {
          commandRequest.strong_auth_grant = strongAuthGrant;
        }
      }
      feedback.stage = "sent";
      feedback.effect = `prompt ${normalizedMode} request sent; waiting for ack`;
      state.lastPromptFeedback = feedback;
      sendJson({
        type: "prompt_control",
        command: {
          mode: normalizedMode,
          request: commandRequest
        }
      });
      render();
    }
  });
  render();
  return { ok: true, feedback: snapshotSemanticFeedback(feedback) };
}
function gameplayActionRequiresActorAgent(actionId) {
  return actionId === "claim_agent" || actionId === "release_agent_claim";
}
function normalizeGameplayActionRequest(action) {
  if (!action || typeof action !== "object") {
    return null;
  }
  const normalized = {
    ...action,
    protocol_action: action.protocol_action || action.protocolAction || null,
    action_id: action.action_id || action.actionId || null,
    target_agent_id: action.target_agent_id || action.targetAgentId || null,
    disabled_reason: action.disabled_reason || action.disabledReason || null
  };
  return normalized;
}
function resolveGameplayActionRequest(actionOrId) {
  if (typeof actionOrId === "string") {
    const actions = Array.isArray(state.snapshot?.player_gameplay?.available_actions) ? state.snapshot.player_gameplay.available_actions : [];
    return actions.find((action) => action?.action_id === actionOrId) || null;
  }
  if (!actionOrId || typeof actionOrId !== "object") {
    return null;
  }
  if (typeof actionOrId.actionId === "string" && actionOrId.actionId.trim()) {
    const resolved = resolveGameplayActionRequest(actionOrId.actionId.trim());
    if (resolved) {
      return resolved;
    }
  }
  return normalizeGameplayActionRequest(actionOrId);
}
function sendGameplayAction(actionOrId) {
  const action = resolveGameplayActionRequest(actionOrId);
  if (!action) {
    return { ok: false, reason: "gameplay action is unavailable in the current snapshot" };
  }
  const protocolAction = String(action.protocol_action || "").trim();
  if (protocolAction === "request_snapshot" || protocolAction === "world.request_snapshot") {
    requestSnapshotSafe();
    state.lastGameplayActionFeedback = {
      id: nextRequestId(),
      kind: "gameplay_action",
      action: action.action_id || "request_snapshot",
      agentId: action.target_agent_id || null,
      accepted: true,
      ok: true,
      stage: "ack",
      reason: null,
      effect: "snapshot refresh requested",
      response: {
        action_id: action.action_id || "request_snapshot",
        target_agent_id: action.target_agent_id || "",
        accepted_at_tick: state.logicalTime,
        message: "snapshot refresh requested"
      }
    };
    render();
    return { ok: true, feedback: snapshotSemanticFeedback(state.lastGameplayActionFeedback) };
  }
  if (protocolAction === "live_control.step") {
    return { ok: true, feedback: sendControl("step", { count: 1 }) };
  }
  if (protocolAction === "live_control.play") {
    return { ok: true, feedback: sendControl("play", null) };
  }
  if (protocolAction !== "gameplay_action.submit") {
    return { ok: false, reason: `unsupported gameplay action protocol: ${protocolAction || "(empty)"}` };
  }
  const actionId = String(action.action_id || "").trim();
  const targetAgentId = String(action.target_agent_id || "").trim();
  if (!actionId || !targetAgentId) {
    return { ok: false, reason: "gameplay_action.submit requires action_id and target_agent_id" };
  }
  const disabledReason = String(action.disabled_reason || "").trim();
  if (disabledReason) {
    return { ok: false, reason: disabledReason };
  }
  const feedback = createSemanticFeedback("gameplay_action", actionId, targetAgentId, {
    effect: "queued for signing and send",
    targetAgentId,
    protocolAction
  });
  state.lastGameplayActionFeedback = feedback;
  render();
  void (async () => {
    try {
      await ensureHostedPlayerAuthAvailable();
      assertSemanticCapability(actionId);
      feedback.stage = "registering";
      feedback.effect = "registering player session";
      render();
      await ensureRegisteredPlayerSession(targetAgentId);
      feedback.stage = "signing";
      feedback.effect = "building auth proof";
      render();
      const request = {
        action_id: actionId,
        target_agent_id: targetAgentId,
        player_id: state.auth.playerId,
        public_key: state.auth.publicKey
      };
      if (gameplayActionRequiresActorAgent(actionId)) {
        request.actor_agent_id = state.auth.boundAgentId || targetAgentId;
      }
      request.auth = await buildGameplayActionAuthProof(request, state.auth);
      feedback.stage = "sent";
      feedback.effect = "gameplay action sent; waiting for ack";
      state.lastGameplayActionFeedback = feedback;
      sendJson({
        type: "gameplay_action",
        request
      });
      render();
    } catch (error) {
      feedback.stage = "error";
      feedback.ok = false;
      feedback.accepted = false;
      feedback.reason = String(error);
      feedback.effect = "gameplay action send failed";
      state.lastGameplayActionFeedback = feedback;
      render();
    }
  })();
  return { ok: true, feedback: snapshotSemanticFeedback(feedback) };
}
function handleGameplayActionAck(ack) {
  const feedback = state.lastGameplayActionFeedback || createSemanticFeedback(
    "gameplay_action",
    ack?.action_id || "gameplay_action",
    ack?.target_agent_id || null
  );
  feedback.stage = "ack";
  feedback.ok = true;
  feedback.accepted = true;
  feedback.reason = null;
  feedback.effect = ack?.message || `gameplay action accepted at tick ${Number(ack?.accepted_at_tick || state.logicalTime)}`;
  feedback.response = clone(ack);
  state.lastGameplayActionFeedback = feedback;
  requestSnapshotSafe();
}
function handleGameplayActionError(error) {
  const feedback = state.lastGameplayActionFeedback || createSemanticFeedback(
    "gameplay_action",
    error?.action_id || "gameplay_action",
    error?.target_agent_id || null
  );
  feedback.stage = "error";
  feedback.ok = false;
  feedback.accepted = false;
  feedback.reason = error?.message || error?.code || "gameplay action failed";
  feedback.effect = error?.code || "gameplay action error";
  feedback.response = clone(error);
  state.lastGameplayActionFeedback = feedback;
}
function applyPromptAckLocally(ack) {
  const agentId = ack?.agent_id;
  if (!agentId || !state.snapshot?.model) {
    return;
  }
  if (!state.snapshot.model.agent_prompt_profiles) {
    state.snapshot.model.agent_prompt_profiles = {};
  }
  const current = state.snapshot.model.agent_prompt_profiles[agentId] || { agent_id: agentId };
  const nextProfile = {
    ...current,
    agent_id: agentId,
    version: Number(ack.version || current.version || 0),
    updated_at_tick: Number(ack.updated_at_tick || state.logicalTime),
    updated_by: state.auth.playerId || current.updated_by || ""
  };
  if (!ack.preview) {
    nextProfile.system_prompt_override = state.promptDraft.systemPrompt || null;
    nextProfile.short_term_goal_override = state.promptDraft.shortTermGoal || null;
    nextProfile.long_term_goal_override = state.promptDraft.longTermGoal || null;
  }
  state.snapshot.model.agent_prompt_profiles[agentId] = nextProfile;
  if (selectedAgentId() === agentId) {
    state.promptDraft = {
      agentId,
      currentVersion: nextProfile.version,
      rollbackTargetVersion: Math.max(0, Number(nextProfile.version || 0) - 1),
      updatedBy: nextProfile.updated_by,
      updatedAtTick: nextProfile.updated_at_tick,
      systemPrompt: String(nextProfile.system_prompt_override || ""),
      shortTermGoal: String(nextProfile.short_term_goal_override || ""),
      longTermGoal: String(nextProfile.long_term_goal_override || ""),
      dirty: false
    };
  }
}
function handlePromptControlAck(ack) {
  const feedback = state.lastPromptFeedback || createSemanticFeedback("prompt", "prompt_ack", ack?.agent_id || null);
  const operation = String(ack?.operation || (ack?.preview ? "preview" : "apply"));
  feedback.stage = ack?.preview ? "preview_ack" : operation === "rollback" ? "rollback_ack" : "apply_ack";
  feedback.ok = true;
  feedback.accepted = true;
  feedback.reason = null;
  feedback.effect = ack?.preview ? `prompt preview ready: version=${ack.version}` : operation === "rollback" ? `prompt rolled back via version=${ack.version} → target=${Number(ack?.rolled_back_to_version || 0)}` : `prompt applied: version=${ack.version}`;
  feedback.response = clone(ack);
  state.lastPromptFeedback = feedback;
  if (ack?.preview) {
    return;
  }
  if (operation === "rollback") {
    state.promptDraft.currentVersion = Number(ack?.version || state.promptDraft.currentVersion || 0);
    state.promptDraft.rollbackTargetVersion = Math.max(0, state.promptDraft.currentVersion - 1);
    state.promptDraft.dirty = false;
    requestSnapshotSafe();
    return;
  }
  applyPromptAckLocally(ack);
}
function handlePromptControlError(error) {
  const feedback = state.lastPromptFeedback || createSemanticFeedback("prompt", "prompt_error", error?.agent_id || selectedAgentId());
  feedback.stage = "error";
  feedback.ok = false;
  feedback.accepted = false;
  feedback.reason = error?.message || error?.code || "prompt control failed";
  feedback.effect = error?.code || "prompt control error";
  feedback.response = clone(error);
  state.lastPromptFeedback = feedback;
}
function handleAgentChatAck(ack) {
  const feedback = state.lastChatFeedback || createSemanticFeedback("chat", "agent_chat", ack?.agent_id || null);
  feedback.stage = "ack";
  feedback.ok = true;
  feedback.accepted = true;
  feedback.reason = null;
  feedback.effect = `chat accepted at tick ${Number(ack?.accepted_at_tick || state.logicalTime)}`;
  feedback.response = clone(ack);
  state.lastChatFeedback = feedback;
  pushChatHistory({
    id: `chat-ack-${feedback.id}`,
    source: "player",
    agentId: ack?.agent_id || feedback.agentId || null,
    message: feedback.pendingMessage || "",
    tick: Number(ack?.accepted_at_tick || state.logicalTime || 0),
    speaker: feedback.pendingPlayerId || state.auth.playerId || null,
    playerId: feedback.pendingPlayerId || state.auth.playerId || null,
    targetAgentId: ack?.agent_id || feedback.agentId || null,
    intentSeq: ack?.intent_seq || null
  });
}
function handleAgentChatError(error) {
  const feedback = state.lastChatFeedback || createSemanticFeedback("chat", "agent_chat", error?.agent_id || selectedAgentId());
  feedback.stage = "error";
  feedback.ok = false;
  feedback.accepted = false;
  feedback.reason = error?.message || error?.code || "agent chat failed";
  feedback.effect = error?.code || "agent chat error";
  feedback.response = clone(error);
  state.lastChatFeedback = feedback;
}
function adoptHostedRecoveryAck(ack) {
  if (!ack || !state.auth.available) {
    return;
  }
  const usesLegacyPreviewBootstrap = state.auth.source === "legacy_viewer_auth_bootstrap";
  const hadPendingForceRebind = state.auth.pendingForceRebind === true;
  const previousRequestedAgentId = state.auth.pendingRequestedAgentId;
  const nextBoundAgentId = ack.agent_id || state.auth.boundAgentId || null;
  const nextRequestedAgentId = ack.agent_id || state.auth.pendingRequestedAgentId || state.auth.boundAgentId || null;
  state.auth.syncInFlight = false;
  state.auth.recoveryErrorCode = null;
  state.auth.recoveryErrorMessage = null;
  state.auth.error = null;
  state.auth.revokeReason = null;
  state.auth.revokedBy = null;
  if (ack.player_id) {
    state.auth.playerId = ack.player_id;
  }
  if (ack.session_pubkey) {
    state.auth.publicKey = ack.session_pubkey;
  }
  if (ack.session_epoch != null) {
    state.auth.sessionEpoch = Number(ack.session_epoch);
  }
  state.auth.boundAgentId = nextBoundAgentId;
  state.auth.pendingRequestedAgentId = nextRequestedAgentId;
  state.auth.pendingForceRebind = false;
  if (ack.status === "session_registered" && hadPendingForceRebind) {
    state.auth.rebindNotice = `Player session switched to ${ack.agent_id || previousRequestedAgentId || "requested agent"}.`;
  }
  state.auth.registrationStatus = ack.status === "session_registered" || ack.status === "catch_up_ready" ? "registered" : ack.status === "session_revoked" ? "guest" : "issued";
  state.auth.runtimeStatus = ack.status === "session_revoked" ? "revoked" : nextBoundAgentId ? "registered" : "registered_unbound";
  if (ack.status === "session_revoked") {
    if (usesLegacyPreviewBootstrap) {
      state.auth.registrationStatus = "issued";
      state.auth.runtimeStatus = "revoked";
      state.auth.error = ack.message || "legacy preview player session was revoked";
      state.auth.pendingRequestedAgentId = null;
      state.auth.pendingForceRebind = false;
    } else {
      void releaseHostedPlayerSlot().catch(() => {
      });
      resetHostedPlayerAuthState(
        ack.message || "hosted player session was revoked",
        {
          revokeReason: ack.revoke_reason || ack.message || null,
          revokedBy: ack.revoked_by || null
        }
      );
    }
  } else {
    if (!usesLegacyPreviewBootstrap) {
      persistHostedPlayerSession(state.auth);
      void refreshHostedPlayerLease();
      syncHostedSessionRefreshLoop();
    }
  }
  if (pendingSessionRegisterWaiter && ack.status === "session_registered") {
    const waiter = pendingSessionRegisterWaiter;
    pendingSessionRegisterWaiter = null;
    waiter.resolve(ack);
  }
}
async function recoverHostedSessionFromError(error) {
  if (!canAutoIssueHostedPlayerSession() || state.auth.source === "legacy_viewer_auth_bootstrap") {
    return;
  }
  const code = String(error?.code || "").trim();
  if (recoveryErrorRequiresExplicitRebind(error) && state.auth.pendingRequestedAgentId && !state.auth.pendingForceRebind) {
    await ensureRegisteredPlayerSession(state.auth.pendingRequestedAgentId, { forceRebind: true });
    return;
  }
  if (code === "session_not_found") {
    await ensureRegisteredPlayerSession(latestRequestedAgentId());
    return;
  }
  if (code === "session_revoked") {
    void releaseHostedPlayerSlot().catch(() => {
    });
    resetHostedPlayerAuthState(
      error?.message || code || "hosted player session failed",
      {
        revokeReason: error?.revoke_reason || error?.message || null,
        revokedBy: error?.revoked_by || null
      }
    );
    render();
    return;
  }
  if (["session_key_mismatch", "session_player_id_invalid"].includes(code)) {
    void releaseHostedPlayerSlot().catch(() => {
    });
    resetHostedPlayerAuthState(error?.message || code || "hosted player session failed");
    render();
    await issueHostedPlayerIdentity();
    if (state.auth.available) {
      await ensureRegisteredPlayerSession(latestRequestedAgentId());
    }
  }
}
function handleAuthoritativeRecoveryAck(ack) {
  adoptHostedRecoveryAck(ack);
}
function handleAuthoritativeRecoveryError(error) {
  if (pendingSessionRegisterWaiter && recoveryErrorRequiresExplicitRebind(error) && pendingSessionRegisterWaiter.requestedAgentId && !pendingSessionRegisterWaiter.forceRebind) {
    state.auth.recoveryErrorCode = error?.code || null;
    state.auth.recoveryErrorMessage = error?.message || null;
    state.auth.error = error?.message || error?.code || "authoritative recovery failed";
    state.auth.registrationStatus = "registering";
    state.auth.runtimeStatus = "rebind_retrying";
    state.auth.pendingForceRebind = true;
    state.auth.rebindNotice = `Requested agent ${state.auth.pendingRequestedAgentId || "-"} needs explicit rebind; retrying now.`;
    markPendingSemanticRebind("explicit rebind required; retrying registration for the requested agent");
    render();
    void retryPendingSessionRegisterWaiterWithForceRebind().catch((retryError) => {
      handleAuthoritativeRecoveryError({
        code: "player_bind_failed",
        message: String(retryError)
      });
    });
    return;
  }
  if (!state.auth.available || state.auth.source === "legacy_viewer_auth_bootstrap") {
    clearPendingSessionRegisterWaiter(error?.message || error?.code || "authoritative recovery failed");
    return;
  }
  state.auth.syncInFlight = false;
  state.auth.recoveryErrorCode = error?.code || null;
  state.auth.recoveryErrorMessage = error?.message || null;
  state.auth.error = error?.message || error?.code || "authoritative recovery failed";
  state.auth.revokeReason = error?.revoke_reason || null;
  state.auth.revokedBy = error?.revoked_by || null;
  state.auth.registrationStatus = "issued";
  state.auth.runtimeStatus = error?.code === "session_revoked" ? "revoked" : error?.code === "session_not_found" ? "missing" : "error";
  if (!recoveryErrorRequiresExplicitRebind(error)) {
    state.auth.boundAgentId = null;
  }
  clearPendingSessionRegisterWaiter(error?.message || error?.code || "authoritative recovery failed");
  syncHostedSessionRefreshLoop();
  void recoverHostedSessionFromError(error);
}
function handleViewerMessage(message) {
  switch (message?.type) {
    case "hello_ack":
      state.server = message.server || null;
      state.worldId = message.world_id || null;
      state.controlProfile = message.control_profile || "playback";
      state.debugViewerStatus = "subscribed";
      void ensureHostedPlayerAuthAvailable().then(() => {
        syncHostedPlayerSessionOnConnect();
        render();
      });
      break;
    case "snapshot":
      handleSnapshot(message.snapshot);
      break;
    case "event": {
      addRecentEvent(message.event);
      const chatEntry = extractAgentSpokeEntry(message.event);
      if (chatEntry) {
        pushChatHistory(chatEntry);
      }
      state.logicalTime = Math.max(state.logicalTime, Number(message.event?.time || 0));
      state.tick = state.logicalTime;
      break;
    }
    case "metrics":
      handleMetrics(message.time, message.metrics);
      break;
    case "control_completion_ack":
      handleControlCompletionAck(message.ack);
      break;
    case "prompt_control_ack":
      handlePromptControlAck(message.ack);
      break;
    case "prompt_control_error":
      handlePromptControlError(message.error);
      break;
    case "agent_chat_ack":
      handleAgentChatAck(message.ack);
      break;
    case "agent_chat_error":
      handleAgentChatError(message.error);
      break;
    case "gameplay_action_ack":
      handleGameplayActionAck(message.ack);
      break;
    case "gameplay_action_error":
      handleGameplayActionError(message.error);
      break;
    case "authoritative_recovery_ack":
      handleAuthoritativeRecoveryAck(message.ack);
      break;
    case "authoritative_recovery_error":
      handleAuthoritativeRecoveryError(message.error);
      break;
    case "error":
      reportFatalError(message.message, "viewer");
      break;
  }
  updateControlFeedbackFromProgress();
  render();
}
function attachSocket(ws) {
  ws.addEventListener("open", () => {
    state.connectionStatus = "connected";
    state.debugViewerStatus = "detached";
    state.lastError = null;
    sendJson({ type: "hello", client: "viewer", version: 1 });
    sendJson({ type: "subscribe", streams: ["snapshot", "events", "metrics"], event_kinds: [] });
    sendJson({ type: "request_snapshot" });
    syncHostedSessionRefreshLoop();
    render();
  });
  ws.addEventListener("message", (event) => {
    try {
      const message = JSON.parse(String(event.data || "null"));
      handleViewerMessage(message);
    } catch (error) {
      reportFatalError(String(error), "viewer.parse");
    }
  });
  ws.addEventListener("error", () => {
    reportFatalError("websocket error", "viewer.ws");
  });
  ws.addEventListener("close", () => {
    state.connectionStatus = "connecting";
    state.debugViewerStatus = "detached";
    if (state.auth.available && state.auth.source !== "legacy_viewer_auth_bootstrap") {
      state.auth.syncInFlight = false;
      state.auth.runtimeStatus = "disconnected";
    }
    clearPendingSessionRegisterWaiter("websocket disconnected during player session registration");
    stopHostedSessionRefreshLoop();
    render();
    if (reconnectTimer) {
      window.clearTimeout(reconnectTimer);
    }
    reconnectTimer = window.setTimeout(connect, 1200);
  });
}
function connect() {
  if (socket) {
    try {
      socket.close();
    } catch (_) {
    }
  }
  const params = getSearchParams();
  state.wsUrl = normalizeWsAddr(params.get("ws") || params.get("addr") || DEFAULT_WS_ADDR);
  state.connectionStatus = "connecting";
  render();
  socket = new WebSocket(state.wsUrl);
  attachSocket(socket);
}
function resourceSummary(resources) {
  if (!resources || typeof resources !== "object") {
    return "-";
  }
  return Object.entries(resources).map(([key, value]) => {
    if (value && typeof value === "object") {
      return `${key}:${JSON.stringify(value)}`;
    }
    return `${key}:${value}`;
  }).join(" · ") || "-";
}
function modelLists() {
  const { agents, locations } = entityCollections();
  const keyword = selectedSearch.trim().toLowerCase();
  const filter = (entry, label) => {
    if (!keyword) return true;
    return String(label).toLowerCase().includes(keyword);
  };
  return {
    agents: agents.filter((agent) => filter(agent, `${agent.id} ${agent.location_id}`)).sort((a, b) => String(a.id).localeCompare(String(b.id))),
    locations: locations.filter((location) => filter(location, `${location.id} ${location.name}`)).sort((a, b) => String(a.id).localeCompare(String(b.id)))
  };
}
function connectionBadgeClass() {
  if (state.connectionStatus === "connected") return "badge badge--good";
  if (state.connectionStatus === "error") return "badge badge--bad";
  return "badge badge--warn";
}
function feedbackBadgeClass(feedback) {
  if (!feedback) return "badge";
  if (feedback.stage === "error") return "badge badge--bad";
  if (feedback.ok) return "badge badge--good";
  return "badge badge--warn";
}
function render() {
  renderHook();
}
function requestRender() {
  render();
}
function setStrongAuthApprovalCode(value) {
  state.strongAuth.approvalCode = String(value || "");
  render();
  return {
    ok: true,
    configured: !!state.strongAuth.approvalCode.trim()
  };
}
function installTestApi() {
  if (!isTestApiEnabled()) {
    return;
  }
  window[TEST_API_GLOBAL_NAME] = {
    getState,
    describeControls,
    fillControlExample,
    sendControl,
    sendGameplayAction,
    runSteps,
    setMode,
    focus,
    select,
    sendAgentChat,
    sendPromptControl,
    setPromptOverridesVisible,
    togglePromptOverridesVisible,
    setStrongAuthApprovalCode,
    injectSnapshot,
    logoutHostedPlayerSession,
    retryHostedPlayerIdentityIssue,
    reportFatalError
  };
}
function bootstrap() {
  state.uiLocale = resolveInitialUiLocale();
  state.promptOverridesVisible = resolveStoredPromptOverridesVisibility();
  applyUiLocaleToDocument(state.uiLocale);
  Object.assign(state, detectRendererMeta());
  state.hostedAccess = resolveHostedAccessHint();
  state.auth = resolveViewerAuthState();
  state.wsUrl = initialWsUrl();
  window[RENDER_META_GLOBAL_NAME] = Object.freeze({
    renderMode: state.renderMode,
    rendererClass: state.rendererClass,
    viewerReason: state.viewerReason,
    softwareSafeReason: state.viewerReason,
    renderer: state.renderer,
    vendor: state.vendor,
    webglVersion: state.webglVersion
  });
  installTestApi();
  render();
  void refreshHostedAdmissionState().then(() => render());
  void ensureHostedPlayerAuthAvailable().then(() => render());
  if (shouldConnectViewerWs()) {
    connect();
  } else {
    state.connectionStatus = "disconnected";
  }
}
function updatePixelWorldRuntimeMeta(meta = {}) {
  if (!meta || typeof meta !== "object") {
    return getState();
  }
  if (Object.prototype.hasOwnProperty.call(meta, "runtimeStatus")) {
    state.pixelWorldRuntimeStatus = meta.runtimeStatus || "detached";
  }
  if (Object.prototype.hasOwnProperty.call(meta, "runtimeSource")) {
    state.pixelWorldRuntimeSource = meta.runtimeSource || "detached";
  }
  if (Object.prototype.hasOwnProperty.call(meta, "runtimeModuleUrl")) {
    state.pixelWorldRuntimeModuleUrl = meta.runtimeModuleUrl || null;
  }
  if (Object.prototype.hasOwnProperty.call(meta, "camera")) {
    state.pixelWorldCamera = clone(meta.camera || null);
  }
  if (Object.prototype.hasOwnProperty.call(meta, "fatal")) {
    state.pixelWorldFatal = clone(meta.fatal || null);
  }
  return getState();
}
function initializeSoftwareSafeCore() {
  if (bootstrapped) {
    return;
  }
  bootstrapped = true;
  bootstrap();
}
window.addEventListener("error", (event) => {
  const message = event?.message || event?.error?.message || "window error";
  reportFatalError(message, "window.error");
});
window.addEventListener("unhandledrejection", (event) => {
  const message = event?.reason?.message || String(event?.reason || "unhandled rejection");
  reportFatalError(message, "window.unhandledrejection");
});
function clamp(value, min, max) {
  return Math.min(max, Math.max(min, value));
}
function createInitialCameraState() {
  return {
    zoom: 1,
    pan_x_px: 0,
    pan_y_px: 0
  };
}
function toCanvasPoint(position, worldBounds, width, height, cameraState) {
  if (!position || !worldBounds) {
    return null;
  }
  const safeWidth = Math.max(1, Number(worldBounds.width_cm) || 1);
  const safeDepth = Math.max(1, Number(worldBounds.depth_cm) || 1);
  const normalizedX = clamp(position.x_cm / safeWidth, 0, 1);
  const normalizedY = clamp(position.y_cm / safeDepth, 0, 1);
  const baseX = 20 + normalizedX * Math.max(1, width - 40);
  const baseY = 20 + normalizedY * Math.max(1, height - 40);
  const zoom = Math.max(0.5, Number(cameraState?.zoom) || 1);
  const panX = Number(cameraState?.pan_x_px) || 0;
  const panY = Number(cameraState?.pan_y_px) || 0;
  const centeredX = baseX - width / 2;
  const centeredY = baseY - height / 2;
  return {
    x: width / 2 + centeredX * zoom + panX,
    y: height / 2 + centeredY * zoom + panY
  };
}
function fallbackPointForEntity(id, width, height, cameraState) {
  const baseX = 36 + Math.abs(id.length * 29) % Math.max(40, width - 72);
  const baseY = 44 + Math.abs(id.length * 17) % Math.max(48, height - 88);
  return toCanvasPoint(
    { x_cm: baseX, y_cm: baseY },
    { width_cm: width, depth_cm: height },
    width,
    height,
    cameraState
  );
}
function drawGrid(context, width, height, cameraState) {
  const zoom = Math.max(0.5, Number(cameraState?.zoom) || 1);
  const panX = Number(cameraState?.pan_x_px) || 0;
  const panY = Number(cameraState?.pan_y_px) || 0;
  const gridStep = clamp(24 * zoom, 12, 72);
  const offsetX = (panX % gridStep + gridStep) % gridStep;
  const offsetY = (panY % gridStep + gridStep) % gridStep;
  context.strokeStyle = "rgba(99, 179, 255, 0.10)";
  context.lineWidth = 1;
  for (let x = offsetX; x <= width; x += gridStep) {
    context.beginPath();
    context.moveTo(x + 0.5, 0);
    context.lineTo(x + 0.5, height);
    context.stroke();
  }
  for (let y = offsetY; y <= height; y += gridStep) {
    context.beginPath();
    context.moveTo(0, y + 0.5);
    context.lineTo(width, y + 0.5);
    context.stroke();
  }
}
function drawBridgeFrame(canvas, renderState, cameraState, animationMs) {
  const context = canvas.getContext("2d");
  if (!context) {
    throw new Error("2d canvas context unavailable");
  }
  const width = canvas.width;
  const height = canvas.height;
  context.clearRect(0, 0, width, height);
  context.fillStyle = "#0a121a";
  context.fillRect(0, 0, width, height);
  drawGrid(context, width, height, cameraState);
  for (const location of renderState.locations || []) {
    const point = toCanvasPoint(location.pos, renderState.world_bounds, width, height, cameraState);
    if (!point) {
      continue;
    }
    const pulse = 1 + 0.08 * Math.sin(animationMs / 360 + location.id.length);
    const size = 16 * pulse;
    context.fillStyle = "rgba(110, 231, 183, 0.72)";
    context.fillRect(point.x - size / 2, point.y - size / 2, size, size);
    context.strokeStyle = "rgba(110, 231, 183, 0.95)";
    context.strokeRect(point.x - size / 2, point.y - size / 2, size, size);
  }
  for (const [index, agent] of (renderState.agents || []).entries()) {
    const point = toCanvasPoint(agent.pos, renderState.world_bounds, width, height, cameraState) || fallbackPointForEntity(agent.id, width, height, cameraState);
    const isSelected = renderState.selection?.kind === "agent" && renderState.selection?.id === agent.id;
    const pulse = 1 + 0.12 * Math.sin(animationMs / 240 + index);
    const size = (isSelected ? 15 : 12) * pulse;
    context.fillStyle = isSelected ? "#fbbf24" : "#63b3ff";
    context.fillRect(point.x - size / 2, point.y - size / 2, size, size);
    context.strokeStyle = isSelected ? "#fde68a" : "#c6e4ff";
    context.lineWidth = 2;
    context.strokeRect(point.x - size / 2, point.y - size / 2, size, size);
  }
}
function createPixelWorldBevyBridge({ onEvent, onFatal } = {}) {
  let mountedCanvas = null;
  let lastRenderState = null;
  let hitRegions = [];
  let cameraState = createInitialCameraState();
  let boundPointerDown = null;
  let boundPointerMove = null;
  let boundPointerUp = null;
  let boundWheel = null;
  let boundClick = null;
  let lastHoverId = null;
  let dragState = null;
  let animationFrameId = null;
  let lastAnimationMs = 0;
  function emit(event) {
    onEvent?.(event);
  }
  function emitCameraState() {
    emit({
      type: "camera_state_changed",
      camera: {
        zoom: Number(cameraState.zoom.toFixed(3)),
        pan_x_px: Math.round(cameraState.pan_x_px),
        pan_y_px: Math.round(cameraState.pan_y_px)
      }
    });
  }
  function fatal(error) {
    const normalized = {
      code: "pixel_world_renderer_fatal",
      message: error instanceof Error ? error.message : String(error || "renderer fatal")
    };
    onFatal?.(normalized);
    return normalized;
  }
  function rebuildHitRegions(canvas, renderState) {
    const width = canvas.width;
    const height = canvas.height;
    const nextRegions = [];
    for (const location of renderState.locations || []) {
      const point = toCanvasPoint(location.pos, renderState.world_bounds, width, height, cameraState);
      if (!point) {
        continue;
      }
      nextRegions.push({
        kind: "location",
        id: location.id,
        left: point.x - 8,
        top: point.y - 8,
        right: point.x + 8,
        bottom: point.y + 8
      });
    }
    for (const agent of renderState.agents || []) {
      const point = toCanvasPoint(agent.pos, renderState.world_bounds, width, height, cameraState) || fallbackPointForEntity(agent.id, width, height, cameraState);
      nextRegions.push({
        kind: "agent",
        id: agent.id,
        left: point.x - 8,
        top: point.y - 8,
        right: point.x + 8,
        bottom: point.y + 8
      });
    }
    hitRegions = nextRegions;
  }
  function renderCurrentFrame(animationMs = lastAnimationMs) {
    if (!mountedCanvas || !lastRenderState) {
      return;
    }
    lastAnimationMs = animationMs;
    drawBridgeFrame(mountedCanvas, lastRenderState, cameraState, animationMs);
    rebuildHitRegions(mountedCanvas, lastRenderState);
  }
  function stopAnimationLoop() {
    if (animationFrameId !== null) {
      cancelAnimationFrame(animationFrameId);
      animationFrameId = null;
    }
  }
  function scheduleAnimationLoop() {
    stopAnimationLoop();
    const animate = (animationMs) => {
      animationFrameId = requestAnimationFrame(animate);
      try {
        renderCurrentFrame(animationMs);
      } catch (error) {
        stopAnimationLoop();
        fatal(error);
      }
    };
    animationFrameId = requestAnimationFrame(animate);
  }
  function eventPoint(canvas, event) {
    const rect = canvas.getBoundingClientRect();
    if (!rect.width || !rect.height) {
      return null;
    }
    const scaleX = canvas.width / rect.width;
    const scaleY = canvas.height / rect.height;
    return {
      x: (event.clientX - rect.left) * scaleX,
      y: (event.clientY - rect.top) * scaleY
    };
  }
  function hitTest(point) {
    if (!point) {
      return null;
    }
    for (let index = hitRegions.length - 1; index >= 0; index -= 1) {
      const region = hitRegions[index];
      if (point.x >= region.left && point.x <= region.right && point.y >= region.top && point.y <= region.bottom) {
        return { kind: region.kind, id: region.id };
      }
    }
    return null;
  }
  function detachCanvasEvents() {
    if (!mountedCanvas) {
      return;
    }
    if (boundPointerDown) {
      mountedCanvas.removeEventListener("pointerdown", boundPointerDown);
    }
    if (boundPointerMove) {
      mountedCanvas.removeEventListener("pointermove", boundPointerMove);
      mountedCanvas.removeEventListener("pointerleave", boundPointerMove);
    }
    if (boundPointerUp) {
      mountedCanvas.removeEventListener("pointerup", boundPointerUp);
      mountedCanvas.removeEventListener("pointercancel", boundPointerUp);
    }
    if (boundWheel) {
      mountedCanvas.removeEventListener("wheel", boundWheel);
    }
    if (boundClick) {
      mountedCanvas.removeEventListener("click", boundClick);
    }
    dragState = null;
    mountedCanvas.style.cursor = "default";
    boundPointerDown = null;
    boundPointerMove = null;
    boundPointerUp = null;
    boundWheel = null;
    boundClick = null;
    lastHoverId = null;
  }
  function attachCanvasEvents(canvas) {
    detachCanvasEvents();
    boundPointerDown = (event) => {
      dragState = {
        pointerId: event.pointerId,
        startClientX: event.clientX,
        startClientY: event.clientY,
        startPanX: cameraState.pan_x_px,
        startPanY: cameraState.pan_y_px,
        moved: false
      };
      canvas.style.cursor = "grabbing";
      canvas.setPointerCapture?.(event.pointerId);
    };
    boundPointerMove = (event) => {
      if (dragState && event.pointerId === dragState.pointerId) {
        const deltaX = event.clientX - dragState.startClientX;
        const deltaY = event.clientY - dragState.startClientY;
        if (Math.abs(deltaX) > 1 || Math.abs(deltaY) > 1) {
          dragState.moved = true;
        }
        cameraState = {
          ...cameraState,
          pan_x_px: dragState.startPanX + deltaX,
          pan_y_px: dragState.startPanY + deltaY
        };
        renderCurrentFrame();
        emitCameraState();
        return;
      }
      if (event.type === "pointerleave") {
        if (lastHoverId !== null) {
          lastHoverId = null;
          emit({ type: "hover_entity", selection: null });
        }
        canvas.style.cursor = "default";
        return;
      }
      const hit = hitTest(eventPoint(canvas, event));
      const hoverKey = hit ? `${hit.kind}/${hit.id}` : null;
      if (hoverKey === lastHoverId) {
        return;
      }
      lastHoverId = hoverKey;
      canvas.style.cursor = hit ? "pointer" : "grab";
      emit({ type: "hover_entity", selection: hit });
    };
    boundPointerUp = (event) => {
      if (dragState && event.pointerId === dragState.pointerId) {
        canvas.releasePointerCapture?.(event.pointerId);
        const moved = dragState.moved;
        dragState = null;
        canvas.style.cursor = moved ? "grab" : lastHoverId ? "pointer" : "default";
      }
    };
    boundWheel = (event) => {
      event.preventDefault();
      const nextZoom = clamp(
        cameraState.zoom * (event.deltaY < 0 ? 1.12 : 0.89),
        0.6,
        3.5
      );
      if (Math.abs(nextZoom - cameraState.zoom) < 1e-3) {
        return;
      }
      cameraState = {
        ...cameraState,
        zoom: nextZoom
      };
      renderCurrentFrame();
      emitCameraState();
    };
    boundClick = (event) => {
      if (dragState?.moved) {
        return;
      }
      const hit = hitTest(eventPoint(canvas, event));
      if (!hit) {
        return;
      }
      emit({ type: "select_entity", selection: hit });
    };
    canvas.style.cursor = "grab";
    canvas.addEventListener("pointerdown", boundPointerDown);
    canvas.addEventListener("pointermove", boundPointerMove);
    canvas.addEventListener("pointerleave", boundPointerMove);
    canvas.addEventListener("pointerup", boundPointerUp);
    canvas.addEventListener("pointercancel", boundPointerUp);
    canvas.addEventListener("wheel", boundWheel, { passive: false });
    canvas.addEventListener("click", boundClick);
  }
  return {
    mount(canvas, initialRenderState) {
      if (!(canvas instanceof HTMLCanvasElement)) {
        throw new Error("pixel world bridge mount requires a canvas element");
      }
      mountedCanvas = canvas;
      lastRenderState = initialRenderState;
      cameraState = createInitialCameraState();
      try {
        attachCanvasEvents(canvas);
        renderCurrentFrame(0);
        scheduleAnimationLoop();
      } catch (error) {
        return { status: "fallback", fatal: fatal(error) };
      }
      emit({ type: "canvas_ready" });
      emitCameraState();
      return { status: "ready" };
    },
    update(nextRenderState) {
      lastRenderState = nextRenderState;
      if (!mountedCanvas) {
        return { status: "detached" };
      }
      try {
        renderCurrentFrame();
      } catch (error) {
        return { status: "fallback", fatal: fatal(error) };
      }
      return { status: "ready" };
    },
    unmount() {
      stopAnimationLoop();
      detachCanvasEvents();
      mountedCanvas = null;
      lastRenderState = null;
      hitRegions = [];
      cameraState = createInitialCameraState();
      return { status: "detached" };
    },
    getLastRenderState() {
      return lastRenderState;
    }
  };
}
function resolvePixelWorldRuntimeModuleUrl() {
  if (typeof window !== "undefined" && window.location) {
    return new URL("./pixel-world-bridge/pixel_world_bridge.js", window.location.href).href;
  }
  return "./pixel-world-bridge/pixel_world_bridge.js";
}
const PIXEL_WORLD_WASM_MODULE_URL = resolvePixelWorldRuntimeModuleUrl();
async function tryLoadWasmBridgeModule() {
  try {
    return {
      module: await import(
        /* @vite-ignore */
        PIXEL_WORLD_WASM_MODULE_URL
      ),
      moduleUrl: PIXEL_WORLD_WASM_MODULE_URL
    };
  } catch (_) {
    return null;
  }
}
async function createPixelWorldRuntimeBridge({ onEvent, onFatal } = {}) {
  const runtimeModule = await tryLoadWasmBridgeModule();
  if (runtimeModule?.module?.createPixelWorldBridge) {
    return {
      bridge: await runtimeModule.module.createPixelWorldBridge({ onEvent, onFatal }),
      source: runtimeModule.module.PIXEL_WORLD_RUNTIME_SOURCE || "runtime_module",
      moduleUrl: runtimeModule.moduleUrl
    };
  }
  return {
    bridge: createPixelWorldBevyBridge({ onEvent, onFatal }),
    source: "js_fallback",
    moduleUrl: null
  };
}
var _tmpl$$1 = /* @__PURE__ */ template(`<div class="pixel-world-canvas__callout pixel-world-canvas__callout--goal">`), _tmpl$2$1 = /* @__PURE__ */ template(`<div class="pixel-world-canvas__callout pixel-world-canvas__callout--blocker">`), _tmpl$3$1 = /* @__PURE__ */ template(`<div class=pixel-world-canvas__selection>`), _tmpl$4$1 = /* @__PURE__ */ template(`<div class="pixel-world-canvas pixel-world-canvas--rendered"data-renderer-ready=true><canvas id=pixel-world-embedded-runtime-canvas class=pixel-world-canvas__surface width=960 height=540></canvas><div class=pixel-world-canvas__overlay>`), _tmpl$5$1 = /* @__PURE__ */ template(`<div class=pixel-world-canvas><div class=pixel-world-canvas__grid></div><div class=pixel-world-canvas__overlay>`), _tmpl$6$1 = /* @__PURE__ */ template(`<button class="pixel-world-entity pixel-world-entity--location"><span>`), _tmpl$7$1 = /* @__PURE__ */ template(`<button class="pixel-world-entity pixel-world-entity--agent"><span>`), _tmpl$8$1 = /* @__PURE__ */ template(`<span class=badge>`), _tmpl$9$1 = /* @__PURE__ */ template(`<div class=feedback-detail>`), _tmpl$0$1 = /* @__PURE__ */ template(`<div class="callout callout--warn"><div class=callout__header><div class=callout__title></div></div><div class=callout__body><div class=feedback-summary>`), _tmpl$1$1 = /* @__PURE__ */ template(`<div class="pixel-world-host stack"><div class=pixel-world-host__summary><div class=pixel-world-host__headline></div><div class=feedback-detail></div></div><div class="pixel-world-host__toolbar badge-row"><span class="badge badge--accent"></span><span class="badge badge--accent"></span><span class=badge></span><span class=badge></span><span class=badge></span><span class=badge></span><button type=button></button><button type=button></button><button type=button></button></div><details class=diagnostic><summary></summary><div class=stack style=margin-top:10px><pre class=json>`);
function tr$1(locale, zh, en) {
  return isLocaleZh(locale) ? zh : en;
}
const PIXEL_WORLD_RUNTIME_CANVAS_ID = "pixel-world-embedded-runtime-canvas";
async function waitForRuntimeCanvasAttachment(canvas) {
  for (let attempt = 0; attempt < 12; attempt += 1) {
    if (canvas?.isConnected && document.getElementById(PIXEL_WORLD_RUNTIME_CANVAS_ID) === canvas) {
      return true;
    }
    await new Promise((resolve) => {
      requestAnimationFrame(() => resolve());
    });
  }
  return false;
}
function normalizePosition(pos) {
  if (!pos || typeof pos !== "object") {
    return null;
  }
  const x = Number(pos.x_cm);
  const y = Number(pos.y_cm);
  const z = Number(pos.z_cm);
  if (!Number.isFinite(x) || !Number.isFinite(y) || !Number.isFinite(z)) {
    return null;
  }
  return {
    x_cm: x,
    y_cm: y,
    z_cm: z
  };
}
function buildRecentEventHotspots(events) {
  if (!Array.isArray(events)) {
    return [];
  }
  return events.slice(0, 4).map((event, index) => ({
    id: event?.eventId || event?.event_id || `recent-${index}`,
    title: event?.title || event?.summary || event?.kind || `event-${index}`,
    kind: event?.kind || "recent_event"
  }));
}
function createPixelWorldHostAdapter({
  onSelectEntity,
  onHoverEntity,
  onFatal
}) {
  let bridge = null;
  let runtimeSource = "detached";
  let runtimeModuleUrl = null;
  return {
    async mount(canvas, renderState) {
      const runtime = await createPixelWorldRuntimeBridge({
        onEvent(event) {
          if (event?.type === "canvas_ready") {
            return;
          }
          if (event?.type === "select_entity") {
            onSelectEntity?.(event.selection);
            return;
          }
          if (event?.type === "hover_entity") {
            onHoverEntity?.(event.selection || null);
            return;
          }
          if (event?.type === "camera_state_changed") {
            onFatal?.(null, event.camera || null);
          }
        },
        onFatal
      });
      bridge = runtime.bridge;
      runtimeSource = runtime.source;
      runtimeModuleUrl = runtime.moduleUrl || null;
      const result = bridge.mount(canvas, renderState);
      return {
        status: result?.status || "ready",
        selection: renderState.selection,
        fatal: result?.fatal || null,
        runtimeSource,
        runtimeModuleUrl
      };
    },
    update(renderState) {
      const result = bridge?.update(renderState) || {
        status: "detached"
      };
      return {
        status: result?.status || "ready",
        selection: renderState.selection,
        fatal: result?.fatal || null,
        runtimeSource,
        runtimeModuleUrl
      };
    },
    unmount() {
      const result = bridge?.unmount() || {
        status: "detached"
      };
      bridge = null;
      runtimeSource = "detached";
      runtimeModuleUrl = null;
      return result;
    },
    simulateSelect(selection) {
      if (!selection?.kind || !selection?.id) {
        return;
      }
      onSelectEntity?.(selection);
    },
    simulateHover(selection) {
      onHoverEntity?.(selection || null);
    },
    simulateFatal(message) {
      onFatal?.({
        code: "pixel_world_renderer_fatal",
        message: String(message || "renderer fatal")
      });
    },
    runtimeSource() {
      return runtimeSource;
    },
    runtimeModuleUrl() {
      return runtimeModuleUrl;
    }
  };
}
function buildPixelWorldRenderState(locale = state.uiLocale) {
  const lists = modelLists();
  const gameplay = buildGameplaySummary(locale);
  const worldScaleSurface = buildWorldScaleSurface(locale);
  const snapshot = state.snapshot;
  const selected = clone(state.selectedObject);
  const space = snapshot?.config?.space || null;
  const worldBounds = space ? {
    width_cm: Number(space.width_cm) || 0,
    depth_cm: Number(space.depth_cm) || 0,
    height_cm: Number(space.height_cm) || 0
  } : null;
  const locations = lists.locations.map((location) => ({
    id: location.id,
    label: location.name || location.id,
    pos: normalizePosition(location.pos),
    radius_cm: Number(location?.profile?.radius_cm) || 0,
    resource_summary: resourceSummary(location.resources)
  })).filter((location) => location.pos);
  const agents = lists.agents.map((agent) => ({
    id: agent.id,
    label: agent.name || agent.id,
    location_id: agent.location_id || null,
    pos: normalizePosition(agent.pos || (selected?.id === agent.id ? selected?.pos : null)),
    resource_summary: resourceSummary(agent.resources),
    status_badges: [agent.location_id ? `location=${agent.location_id}` : null, agent.kind ? `kind=${agent.kind}` : null].filter(Boolean)
  }));
  const selection = state.selectedKind && state.selectedId ? {
    kind: state.selectedKind,
    id: state.selectedId
  } : null;
  return {
    locale,
    world_bounds: worldBounds,
    locations,
    agents,
    selection,
    goal_highlight: gameplay?.goalTitle ? {
      title: gameplay.goalTitle,
      objective: gameplay.objective || null
    } : null,
    blocker_highlight: gameplay?.blockerKind || gameplay?.blockerDetail ? {
      kind: gameplay.blockerKind || "blocked",
      detail: gameplay.blockerDetail || null
    } : null,
    recent_event_hotspots: buildRecentEventHotspots(state.recentEvents),
    presentation: {
      world_bounds_label: worldScaleSurface.physicalTruth.worldBoundsLabel,
      marker_truth_note: worldScaleSurface.presentationScale.markerTruthNote
    }
  };
}
function PixelWorldCanvasRenderer(props) {
  let canvasRef;
  createEffect(() => {
    if (!canvasRef) {
      return;
    }
    props.onCanvasMount?.(canvasRef);
  });
  createEffect(() => {
    props.renderState();
    if (!canvasRef) {
      return;
    }
    props.onCanvasUpdate?.();
  });
  return (() => {
    var _el$ = _tmpl$4$1(), _el$2 = _el$.firstChild, _el$3 = _el$2.nextSibling;
    var _ref$ = canvasRef;
    typeof _ref$ === "function" ? use(_ref$, _el$2) : canvasRef = _el$2;
    insert(_el$3, createComponent(Show, {
      get when() {
        return props.renderState().goal_highlight;
      },
      get children() {
        var _el$4 = _tmpl$$1();
        insert(_el$4, () => `${tr$1(props.locale(), "目标", "Goal")}: ${props.renderState().goal_highlight.title}`);
        return _el$4;
      }
    }), null);
    insert(_el$3, createComponent(Show, {
      get when() {
        return props.renderState().blocker_highlight;
      },
      get children() {
        var _el$5 = _tmpl$2$1();
        insert(_el$5, () => `${tr$1(props.locale(), "阻塞", "Blocker")}: ${props.renderState().blocker_highlight.kind}`);
        return _el$5;
      }
    }), null);
    insert(_el$, createComponent(Show, {
      get when() {
        return props.renderState().selection;
      },
      get children() {
        var _el$6 = _tmpl$3$1();
        insert(_el$6, () => `${tr$1(props.locale(), "已选中", "Selected")}: ${props.renderState().selection.kind}/${props.renderState().selection.id}`);
        return _el$6;
      }
    }), null);
    return _el$;
  })();
}
function PixelWorldCanvasPlaceholder(props) {
  return (() => {
    var _el$7 = _tmpl$5$1(), _el$8 = _el$7.firstChild, _el$0 = _el$8.nextSibling;
    insert(_el$7, createComponent(For, {
      get each() {
        return props.renderState().locations.slice(0, 8);
      },
      children: (location, index) => (() => {
        var _el$11 = _tmpl$6$1(), _el$12 = _el$11.firstChild;
        _el$11.$$click = () => props.onSelect({
          kind: "location",
          id: location.id
        });
        _el$11.addEventListener("mouseleave", () => props.onHover(null));
        _el$11.addEventListener("mouseenter", () => props.onHover({
          kind: "location",
          id: location.id
        }));
        insert(_el$12, () => location.label.slice(0, 2).toUpperCase());
        createRenderEffect((_p$) => {
          var _v$ = `${12 + index() % 4 * 21}%`, _v$2 = `${18 + Math.floor(index() / 4) * 26}%`, _v$3 = location.label;
          _v$ !== _p$.e && setStyleProperty(_el$11, "left", _p$.e = _v$);
          _v$2 !== _p$.t && setStyleProperty(_el$11, "top", _p$.t = _v$2);
          _v$3 !== _p$.a && setAttribute(_el$11, "title", _p$.a = _v$3);
          return _p$;
        }, {
          e: void 0,
          t: void 0,
          a: void 0
        });
        return _el$11;
      })()
    }), _el$0);
    insert(_el$7, createComponent(For, {
      get each() {
        return props.renderState().agents.slice(0, 10);
      },
      children: (agent, index) => (() => {
        var _el$13 = _tmpl$7$1(), _el$14 = _el$13.firstChild;
        _el$13.$$click = () => props.onSelect({
          kind: "agent",
          id: agent.id
        });
        _el$13.addEventListener("mouseleave", () => props.onHover(null));
        _el$13.addEventListener("mouseenter", () => props.onHover({
          kind: "agent",
          id: agent.id
        }));
        insert(_el$14, () => agent.label.slice(0, 1).toUpperCase());
        createRenderEffect((_p$) => {
          var _v$4 = `${18 + index() % 5 * 15}%`, _v$5 = `${14 + Math.floor(index() / 5) * 22}%`, _v$6 = agent.label;
          _v$4 !== _p$.e && setStyleProperty(_el$13, "left", _p$.e = _v$4);
          _v$5 !== _p$.t && setStyleProperty(_el$13, "top", _p$.t = _v$5);
          _v$6 !== _p$.a && setAttribute(_el$13, "title", _p$.a = _v$6);
          return _p$;
        }, {
          e: void 0,
          t: void 0,
          a: void 0
        });
        return _el$13;
      })()
    }), _el$0);
    insert(_el$7, createComponent(Show, {
      get when() {
        return props.renderState().selection;
      },
      get children() {
        var _el$9 = _tmpl$3$1();
        insert(_el$9, () => `${tr$1(props.locale(), "已选中", "Selected")}: ${props.renderState().selection.kind}/${props.renderState().selection.id}`);
        return _el$9;
      }
    }), _el$0);
    insert(_el$0, createComponent(Show, {
      get when() {
        return props.renderState().goal_highlight;
      },
      get children() {
        var _el$1 = _tmpl$$1();
        insert(_el$1, () => `${tr$1(props.locale(), "目标", "Goal")}: ${props.renderState().goal_highlight.title}`);
        return _el$1;
      }
    }), null);
    insert(_el$0, createComponent(Show, {
      get when() {
        return props.renderState().blocker_highlight;
      },
      get children() {
        var _el$10 = _tmpl$2$1();
        insert(_el$10, () => `${tr$1(props.locale(), "阻塞", "Blocker")}: ${props.renderState().blocker_highlight.kind}`);
        return _el$10;
      }
    }), null);
    createRenderEffect(() => setAttribute(_el$7, "data-renderer-ready", props.ready() ? "true" : "false"));
    return _el$7;
  })();
}
function PixelWorldHost(props) {
  const locale = () => props.locale ?? state.uiLocale;
  const renderState = createMemo(() => buildPixelWorldRenderState(locale()));
  const [rendererStatus, setRendererStatus] = createSignal("booting");
  const [rendererFatal, setRendererFatal] = createSignal(null);
  const [hoverSelection, setHoverSelection] = createSignal(null);
  const [runtimeSource, setRuntimeSource] = createSignal("loading");
  const [cameraState, setCameraState] = createSignal(null);
  const adapter = createMemo(() => createPixelWorldHostAdapter({
    onSelectEntity(selection) {
      applySelection(selection);
    },
    onHoverEntity(selection) {
      setHoverSelection(selection);
    },
    onFatal(fatal, nextCameraState) {
      if (nextCameraState) {
        setCameraState(nextCameraState);
        updatePixelWorldRuntimeMeta({
          runtimeStatus: rendererStatus(),
          runtimeSource: runtimeSource(),
          runtimeModuleUrl: adapter().runtimeModuleUrl(),
          camera: nextCameraState,
          fatal: rendererFatal()
        });
        return;
      }
      setRendererFatal(fatal);
      setRendererStatus("fallback");
      updatePixelWorldRuntimeMeta({
        runtimeStatus: "fallback",
        runtimeSource: runtimeSource(),
        runtimeModuleUrl: adapter().runtimeModuleUrl(),
        camera: cameraState(),
        fatal
      });
      reportFatalError(fatal.message, "pixel_world_host");
    }
  }));
  let mountedCanvas = null;
  function applyRendererUpdate() {
    const result = adapter().update(renderState());
    if (result?.fatal) {
      setRendererFatal(result.fatal);
    }
    setRendererStatus(result?.status || "ready");
    setRuntimeSource(result?.runtimeSource || adapter().runtimeSource());
    updatePixelWorldRuntimeMeta({
      runtimeStatus: result?.status || "ready",
      runtimeSource: result?.runtimeSource || adapter().runtimeSource(),
      runtimeModuleUrl: result?.runtimeModuleUrl || adapter().runtimeModuleUrl(),
      camera: cameraState(),
      fatal: result?.fatal || rendererFatal()
    });
  }
  async function setReadyMode() {
    if (!mountedCanvas) {
      const fatal = {
        code: "pixel_world_renderer_mount_missing_canvas",
        message: "pixel world canvas is not mounted yet"
      };
      setRendererFatal(fatal);
      setRendererStatus("fallback");
      setRuntimeSource("detached");
      updatePixelWorldRuntimeMeta({
        runtimeStatus: "fallback",
        runtimeSource: "detached",
        runtimeModuleUrl: null,
        camera: null,
        fatal
      });
      return;
    }
    setRendererFatal(null);
    setRendererStatus("booting");
    setRuntimeSource("loading");
    const attached = await waitForRuntimeCanvasAttachment(mountedCanvas);
    if (!attached) {
      const fatal = {
        code: "pixel_world_renderer_canvas_detached",
        message: "pixel world runtime canvas never became queryable in document"
      };
      setRendererFatal(fatal);
      setRendererStatus("fallback");
      setRuntimeSource("detached");
      updatePixelWorldRuntimeMeta({
        runtimeStatus: "fallback",
        runtimeSource: "detached",
        runtimeModuleUrl: null,
        camera: cameraState(),
        fatal
      });
      return;
    }
    const result = await adapter().mount(mountedCanvas, renderState());
    if (result?.fatal) {
      setRendererFatal(result.fatal);
    }
    setRendererStatus(result?.status || "ready");
    setRuntimeSource(result?.runtimeSource || adapter().runtimeSource());
    updatePixelWorldRuntimeMeta({
      runtimeStatus: result?.status || "ready",
      runtimeSource: result?.runtimeSource || adapter().runtimeSource(),
      runtimeModuleUrl: result?.runtimeModuleUrl || adapter().runtimeModuleUrl(),
      camera: cameraState(),
      fatal: result?.fatal || null
    });
  }
  function setFallbackMode() {
    adapter().unmount();
    setRendererStatus("fallback");
    setRuntimeSource("detached");
    setCameraState(null);
    updatePixelWorldRuntimeMeta({
      runtimeStatus: "fallback",
      runtimeSource: "detached",
      runtimeModuleUrl: null,
      camera: null,
      fatal: rendererFatal()
    });
  }
  function simulateFatal() {
    adapter().simulateFatal("simulated embedded renderer fatal fallback");
  }
  onCleanup(() => {
    adapter().unmount();
    updatePixelWorldRuntimeMeta({
      runtimeStatus: "detached",
      runtimeSource: "detached",
      runtimeModuleUrl: null,
      camera: null,
      fatal: null
    });
  });
  return (() => {
    var _el$15 = _tmpl$1$1(), _el$16 = _el$15.firstChild, _el$17 = _el$16.firstChild, _el$18 = _el$17.nextSibling, _el$19 = _el$16.nextSibling, _el$20 = _el$19.firstChild, _el$21 = _el$20.nextSibling, _el$22 = _el$21.nextSibling, _el$23 = _el$22.nextSibling, _el$24 = _el$23.nextSibling, _el$25 = _el$24.nextSibling, _el$29 = _el$25.nextSibling, _el$30 = _el$29.nextSibling, _el$31 = _el$30.nextSibling, _el$38 = _el$19.nextSibling, _el$39 = _el$38.firstChild, _el$40 = _el$39.nextSibling, _el$41 = _el$40.firstChild;
    insert(_el$17, () => tr$1(locale(), "嵌入式像素世界层（Host Skeleton）", "Embedded Pixel World Layer (Host Skeleton)"));
    insert(_el$18, () => tr$1(locale(), "当前已接入 host-side render DTO、嵌入式 canvas、轻量拖拽缩放和事件回传。后续 Bevy wasm 将接管这个渲染面，但不接管 auth/chat/prompt/control 主链。", "This now wires the host-side render DTO, embedded canvas, light pan-zoom interaction, and event callbacks. Future Bevy wasm will take over this render surface without taking over auth/chat/prompt/control ownership."));
    insert(_el$20, () => `locations=${renderState().locations.length}`);
    insert(_el$21, () => `agents=${renderState().agents.length}`);
    insert(_el$22, () => `hotspots=${renderState().recent_event_hotspots.length}`);
    insert(_el$23, () => renderState().world_bounds ? "world_bounds=ready" : "world_bounds=missing");
    insert(_el$24, () => `renderer=${rendererStatus()}`);
    insert(_el$25, () => `runtime=${runtimeSource()}`);
    insert(_el$19, createComponent(Show, {
      get when() {
        return cameraState();
      },
      get children() {
        var _el$26 = _tmpl$8$1();
        insert(_el$26, () => `zoom=${cameraState().zoom.toFixed(2)}`);
        return _el$26;
      }
    }), _el$29);
    insert(_el$19, createComponent(Show, {
      get when() {
        return cameraState();
      },
      get children() {
        var _el$27 = _tmpl$8$1();
        insert(_el$27, () => `pan=${cameraState().pan_x_px},${cameraState().pan_y_px}`);
        return _el$27;
      }
    }), _el$29);
    insert(_el$19, createComponent(Show, {
      get when() {
        return hoverSelection();
      },
      get children() {
        var _el$28 = _tmpl$8$1();
        insert(_el$28, () => `hover=${hoverSelection().kind}/${hoverSelection().id}`);
        return _el$28;
      }
    }), _el$29);
    _el$29.$$click = () => {
      void setReadyMode();
    };
    insert(_el$29, () => tr$1(locale(), "重新挂载嵌入式 Renderer", "Reattach Embedded Renderer"));
    _el$30.$$click = simulateFatal;
    insert(_el$30, () => tr$1(locale(), "模拟 Renderer Fatal", "Simulate Renderer Fatal"));
    _el$31.$$click = setFallbackMode;
    insert(_el$31, () => tr$1(locale(), "切回 Host Fallback", "Back To Host Fallback"));
    insert(_el$15, createComponent(Show, {
      get when() {
        return rendererStatus() !== "fallback";
      },
      get children() {
        return createComponent(PixelWorldCanvasRenderer, {
          locale,
          renderState,
          onFatal: (message) => adapter().simulateFatal(message),
          onCanvasMount: (canvas) => {
            mountedCanvas = canvas;
            if (rendererStatus() !== "ready") {
              void setReadyMode();
            }
          },
          onCanvasUpdate: () => {
            if (rendererStatus() === "ready") {
              applyRendererUpdate();
            }
          }
        });
      }
    }), _el$38);
    insert(_el$15, createComponent(Show, {
      get when() {
        return rendererStatus() === "fallback";
      },
      get children() {
        var _el$32 = _tmpl$0$1(), _el$33 = _el$32.firstChild, _el$34 = _el$33.firstChild, _el$35 = _el$33.nextSibling, _el$36 = _el$35.firstChild;
        insert(_el$34, () => tr$1(locale(), "Renderer 未接管", "Renderer Not Attached"));
        insert(_el$36, () => tr$1(locale(), "嵌入式 renderer 启动失败，页面已退回 host fallback 模式。正式玩法摘要、目标和明细主链继续可用。", "The embedded renderer failed to attach, so the page returned to host fallback mode. Formal gameplay summary, targets, and details remain available."));
        insert(_el$35, createComponent(Show, {
          get when() {
            return rendererFatal();
          },
          get children() {
            var _el$37 = _tmpl$9$1();
            insert(_el$37, () => `${rendererFatal().code}: ${rendererFatal().message}`);
            return _el$37;
          }
        }), null);
        return _el$32;
      }
    }), _el$38);
    insert(_el$15, createComponent(Show, {
      get when() {
        return rendererStatus() !== "ready";
      },
      get children() {
        return createComponent(PixelWorldCanvasPlaceholder, {
          locale,
          renderState,
          ready: () => false,
          onSelect: (selection) => adapter().simulateSelect(selection),
          onHover: (selection) => adapter().simulateHover(selection)
        });
      }
    }), _el$38);
    insert(_el$39, () => tr$1(locale(), "展开 Render DTO", "Expand Render DTO"));
    insert(_el$41, () => JSON.stringify(renderState(), null, 2));
    return _el$15;
  })();
}
delegateEvents(["click"]);
var _tmpl$ = /* @__PURE__ */ template(`<span>`), _tmpl$2 = /* @__PURE__ */ template(`<div class=empty>`), _tmpl$3 = /* @__PURE__ */ template(`<pre class=json>`), _tmpl$4 = /* @__PURE__ */ template(`<div class=feedback-detail>`), _tmpl$5 = /* @__PURE__ */ template(`<details class=diagnostic><summary></summary><div class=stack style=margin-top:10px>`), _tmpl$6 = /* @__PURE__ */ template(`<div class=feedback-card><div class=badge-row></div><div class=feedback-summary>`), _tmpl$7 = /* @__PURE__ */ template(`<div class=badge-row style=margin-top:8px>`), _tmpl$8 = /* @__PURE__ */ template(`<div class=metric><div class=metric__label></div><div class=metric__value>`), _tmpl$9 = /* @__PURE__ */ template(`<div class=event-card__meta>`), _tmpl$0 = /* @__PURE__ */ template(`<div><div class=event-card__title><span>`), _tmpl$1 = /* @__PURE__ */ template(`<div class=panel__eyebrow>`), _tmpl$10 = /* @__PURE__ */ template(`<div class=panel__meta-copy>`), _tmpl$11 = /* @__PURE__ */ template(`<div><div class=panel__header><div class=stack style=gap:4px><div class=panel__title></div></div></div><div class="panel__body stack">`), _tmpl$12 = /* @__PURE__ */ template(`<div><div class=callout__header><div class=callout__title></div></div><div class=callout__body>`), _tmpl$13 = /* @__PURE__ */ template(`<div class=feedback-summary>`), _tmpl$14 = /* @__PURE__ */ template(`<div class=badge-row>`), _tmpl$15 = /* @__PURE__ */ template(`<details class=entry-menu><summary class=entry-menu__toggle></summary><div class="entry-menu__panel stack"><div><div class=panel__title style=margin-bottom:10px></div><div class=feedback-detail></div></div><div class=toolbar><button data-locale=zh>中文</button><button data-locale=en>English</button></div><div class=badge-row></div><div class=feedback-detail>`), _tmpl$16 = /* @__PURE__ */ template(`<div class=stage-hero><div class=stage-hero__topline><div class=stack style=gap:10px><div class=stage-hero__eyebrow></div><div class=stage-hero__title></div><div class=stage-hero__lede></div></div></div><div class=hero-focus-grid><div class=hero-focus-card><div class=hero-focus-card__label></div><div></div><div class=hero-focus-card__detail></div></div><div class=hero-focus-card><div class=hero-focus-card__label></div><div class=hero-focus-card__value></div><div class=hero-focus-card__detail></div></div><div class=hero-focus-card><div class=hero-focus-card__label></div><div class="hero-focus-card__value hero-focus-card__value--body">`), _tmpl$17 = /* @__PURE__ */ template(`<nav class=mobile-rail><a class=mobile-rail__link href=#viewer-stage-panel></a><a class=mobile-rail__link href=#viewer-targets-panel></a><a class=mobile-rail__link href=#viewer-details-panel>`), _tmpl$18 = /* @__PURE__ */ template(`<div class=stack><div class=field><label for=entity-search></label><input id=entity-search type=search></div><div><div class=panel__title style=margin-bottom:10px></div><div class=list></div></div><div><div class=panel__title style=margin-bottom:10px></div><div class=list>`), _tmpl$19 = /* @__PURE__ */ template(`<button class=list-item data-select-kind=agent><div class=list-item__title></div><div class=list-item__meta>`), _tmpl$20 = /* @__PURE__ */ template(`<button class=list-item data-select-kind=location><div class=list-item__title></div><div class=list-item__meta>`), _tmpl$21 = /* @__PURE__ */ template(`<div class=toolbar><button data-auth-action=retry-issue>`), _tmpl$22 = /* @__PURE__ */ template(`<div class=toolbar><button data-auth-action=logout>`), _tmpl$23 = /* @__PURE__ */ template(`<button data-auth-action=retry-issue>`), _tmpl$24 = /* @__PURE__ */ template(`<button data-auth-action=logout>`), _tmpl$25 = /* @__PURE__ */ template(`<div class=event-list>`), _tmpl$26 = /* @__PURE__ */ template(`<div class=stack><details class="panel diagnostic-surface"><summary class="panel__header diagnostic-surface__summary"><div class=diagnostic-surface__title><div class=panel__title></div><div class=diagnostic-surface__meta></div></div><div class=badge-row></div></summary><div class="panel__body stack"><div class=badge-row></div><div class=badge-row></div><div class=toolbar></div><div class=summary-grid></div><div><div class=panel__title style=margin-bottom:10px></div><div class=event-list>`), _tmpl$27 = /* @__PURE__ */ template(`<div><div class=panel__title style=margin-bottom:10px></div><div class=action-grid>`), _tmpl$28 = /* @__PURE__ */ template(`<div class=toolbar><button>`), _tmpl$29 = /* @__PURE__ */ template(`<div class=field><label for=agent-chat-message></label><textarea id=agent-chat-message rows=4>`), _tmpl$30 = /* @__PURE__ */ template(`<div class=toolbar><button data-chat-send=1>`), _tmpl$31 = /* @__PURE__ */ template(`<div><div class=panel__title style=margin-bottom:10px></div><div class=event-list>`), _tmpl$32 = /* @__PURE__ */ template(`<div class=toolbar><button data-prompt-visibility-toggle=1>`), _tmpl$33 = /* @__PURE__ */ template(`<div class=field><label for=strong-auth-approval-code></label><input id=strong-auth-approval-code type=password autocomplete=off>`), _tmpl$34 = /* @__PURE__ */ template(`<div class=field><label for=prompt-system></label><textarea id=prompt-system rows=4>`), _tmpl$35 = /* @__PURE__ */ template(`<div class=field><label for=prompt-short></label><textarea id=prompt-short rows=3>`), _tmpl$36 = /* @__PURE__ */ template(`<div class=field><label for=prompt-long></label><textarea id=prompt-long rows=3>`), _tmpl$37 = /* @__PURE__ */ template(`<div class=toolbar><button data-prompt-action=preview></button><button data-prompt-action=apply>`), _tmpl$38 = /* @__PURE__ */ template(`<div class=toolbar><div class=field style=margin:0;min-width:180px;flex:1><label for=prompt-rollback-version></label><input id=prompt-rollback-version type=number min=0 step=1></div><button data-prompt-action=rollback>`), _tmpl$39 = /* @__PURE__ */ template(`<div class=toolbar><button disabled>`), _tmpl$40 = /* @__PURE__ */ template(`<div class=stack><div class=badge-row></div><div class=badge-row>`), _tmpl$41 = /* @__PURE__ */ template(`<div><div class=panel__title style=margin-bottom:10px;color:var(--bad)></div><pre class=json>`), _tmpl$42 = /* @__PURE__ */ template(`<div class=stack><div class=badge-row></div><div><div class=panel__title style=margin-bottom:10px></div><div class=badge-row></div><div class=stack style=margin-top:10px><div class=feedback-detail></div><div class=feedback-detail></div><div><div class=panel__title style=margin-bottom:10px></div><div class=event-list>`), _tmpl$43 = /* @__PURE__ */ template(`<div class=feedback-detail>=`), _tmpl$44 = /* @__PURE__ */ template(`<section class="panel panel--targets"id=viewer-targets-panel><div class="panel__header panel__header--stack"><div class=panel__eyebrow></div><div class=panel__title></div><div class=panel__meta-copy></div></div><div class=panel__body>`), _tmpl$45 = /* @__PURE__ */ template(`<section class="panel panel--stage"id=viewer-stage-panel><div class="panel__body panel__body--stage"><div class=stack>`), _tmpl$46 = /* @__PURE__ */ template(`<section class="panel panel--details"id=viewer-details-panel><div class="panel__header panel__header--stack"><div class=panel__eyebrow></div><div class=panel__title></div><div class=panel__meta-copy></div></div><div class=panel__body>`);
function uiLocale() {
  return state.uiLocale;
}
function tr(locale, zh, en) {
  return isLocaleZh(locale) ? zh : en;
}
function localeCode(locale) {
  return isLocaleZh(locale) ? "zh" : "en";
}
function buildViewerEntryUrls(locale) {
  const softwareSafeUrl = new URL(window.location.href);
  softwareSafeUrl.searchParams.set("locale", localeCode(locale));
  softwareSafeUrl.searchParams.delete("language");
  return {
    softwareSafeUrl: softwareSafeUrl.toString()
  };
}
function Badge(props) {
  return (() => {
    var _el$ = _tmpl$();
    insert(_el$, () => props.children);
    createRenderEffect(() => className(_el$, props.class ?? "badge"));
    return _el$;
  })();
}
function EmptyState(props) {
  return (() => {
    var _el$2 = _tmpl$2();
    insert(_el$2, () => props.children);
    createRenderEffect((_$p) => style(_el$2, props.style, _$p));
    return _el$2;
  })();
}
function JsonBlock(props) {
  return (() => {
    var _el$3 = _tmpl$3();
    insert(_el$3, () => JSON.stringify(props.value, null, 2));
    return _el$3;
  })();
}
function DiagnosticDetails(props) {
  const locale = () => props.locale ?? uiLocale();
  const [isOpen, setIsOpen] = createSignal(false);
  const resolvedValue = () => typeof props.value === "function" ? props.value() : props.value;
  return (() => {
    var _el$4 = _tmpl$5(), _el$5 = _el$4.firstChild, _el$6 = _el$5.nextSibling;
    _el$4.addEventListener("toggle", (event) => setIsOpen(event.currentTarget.open));
    insert(_el$5, () => props.label ?? tr(locale(), "原始诊断", "Raw diagnostics"));
    insert(_el$6, createComponent(Show, {
      get when() {
        return props.note;
      },
      get children() {
        var _el$7 = _tmpl$4();
        insert(_el$7, () => props.note);
        return _el$7;
      }
    }), null);
    insert(_el$6, createComponent(Show, {
      get when() {
        return isOpen();
      },
      get children() {
        return createComponent(JsonBlock, {
          get value() {
            return resolvedValue();
          }
        });
      }
    }), null);
    return _el$4;
  })();
}
function FeedbackCard(props) {
  return (() => {
    var _el$8 = _tmpl$6(), _el$9 = _el$8.firstChild, _el$0 = _el$9.nextSibling;
    insert(_el$9, createComponent(Badge, {
      get ["class"]() {
        return props.display.badgeClass;
      },
      get children() {
        return props.display.label;
      }
    }), null);
    insert(_el$9, createComponent(Show, {
      get when() {
        return props.display.code;
      },
      get children() {
        return createComponent(Badge, {
          get children() {
            return `code=${props.display.code}`;
          }
        });
      }
    }), null);
    insert(_el$0, () => props.display.summary);
    insert(_el$8, createComponent(Show, {
      get when() {
        return props.display.detail;
      },
      get children() {
        var _el$1 = _tmpl$4();
        insert(_el$1, () => props.display.detail);
        return _el$1;
      }
    }), null);
    insert(_el$8, createComponent(Show, {
      get when() {
        return props.feedback;
      },
      get children() {
        return createComponent(DiagnosticDetails, {
          get value() {
            return props.feedback;
          }
        });
      }
    }), null);
    return _el$8;
  })();
}
function MetricCard(props) {
  return (() => {
    var _el$10 = _tmpl$8(), _el$11 = _el$10.firstChild, _el$12 = _el$11.nextSibling;
    insert(_el$11, () => props.label);
    insert(_el$12, () => props.value);
    insert(_el$10, createComponent(Show, {
      get when() {
        return props.children;
      },
      get children() {
        var _el$13 = _tmpl$7();
        insert(_el$13, () => props.children);
        return _el$13;
      }
    }), null);
    return _el$10;
  })();
}
function EventCard(props) {
  return (() => {
    var _el$14 = _tmpl$0(), _el$15 = _el$14.firstChild, _el$16 = _el$15.firstChild;
    insert(_el$16, () => props.title);
    insert(_el$15, createComponent(Show, {
      get when() {
        return props.badge;
      },
      get children() {
        var _el$17 = _tmpl$();
        insert(_el$17, () => props.badge);
        createRenderEffect(() => className(_el$17, props.badgeClass ?? "badge"));
        return _el$17;
      }
    }), null);
    insert(_el$14, createComponent(Show, {
      get when() {
        return props.meta;
      },
      get children() {
        var _el$18 = _tmpl$9();
        insert(_el$18, () => props.meta);
        return _el$18;
      }
    }), null);
    insert(_el$14, () => props.children, null);
    createRenderEffect(() => className(_el$14, props.class ?? "event-card"));
    return _el$14;
  })();
}
function PanelSection(props) {
  return (() => {
    var _el$19 = _tmpl$11(), _el$20 = _el$19.firstChild, _el$21 = _el$20.firstChild, _el$23 = _el$21.firstChild, _el$25 = _el$20.nextSibling;
    insert(_el$21, createComponent(Show, {
      get when() {
        return props.eyebrow;
      },
      get children() {
        var _el$22 = _tmpl$1();
        insert(_el$22, () => props.eyebrow);
        return _el$22;
      }
    }), _el$23);
    insert(_el$23, () => props.title);
    insert(_el$21, createComponent(Show, {
      get when() {
        return props.meta;
      },
      get children() {
        var _el$24 = _tmpl$10();
        insert(_el$24, () => props.meta);
        return _el$24;
      }
    }), null);
    insert(_el$25, () => props.children);
    createRenderEffect(() => className(_el$19, `panel panel--nested ${props.class ?? ""}`));
    return _el$19;
  })();
}
function CalloutCard(props) {
  return (() => {
    var _el$26 = _tmpl$12(), _el$27 = _el$26.firstChild, _el$28 = _el$27.firstChild, _el$29 = _el$27.nextSibling;
    insert(_el$28, () => props.title);
    insert(_el$27, createComponent(Show, {
      get when() {
        return props.badge;
      },
      get children() {
        return createComponent(Badge, {
          get ["class"]() {
            return props.badgeClass ?? "badge badge--warn";
          },
          get children() {
            return props.badge;
          }
        });
      }
    }), null);
    insert(_el$29, () => props.children);
    createRenderEffect(() => className(_el$26, `callout ${props.variant === "warn" ? "callout--warn" : ""}`));
    return _el$26;
  })();
}
function EmptyEntityRecoveryCard(props) {
  const locale = () => props.locale ?? uiLocale();
  const gameplay = () => typeof props.gameplay === "function" ? props.gameplay() : props.gameplay;
  return createComponent(CalloutCard, {
    get title() {
      return props.title ?? tr(locale(), "当前快照没有可继续游玩的实体", "Current Snapshot Has No Playable Entities");
    },
    get badge() {
      return gameplay()?.blockerKind || "blocked";
    },
    badgeClass: "badge badge--warn",
    variant: "warn",
    get children() {
      return [(() => {
        var _el$30 = _tmpl$13();
        insert(_el$30, () => gameplay()?.blockerDetail || tr(locale(), "runtime 已发布玩法摘要，但当前快照还没有可选 Agent 或地点。", "Runtime published gameplay summary, but the current snapshot still has no selectable agents or locations."));
        return _el$30;
      })(), createComponent(Show, {
        get when() {
          return gameplay()?.nextStepHint;
        },
        get children() {
          var _el$31 = _tmpl$4();
          insert(_el$31, () => gameplay().nextStepHint);
          return _el$31;
        }
      }), createComponent(Show, {
        get when() {
          return gameplay()?.entityCounts;
        },
        get children() {
          var _el$32 = _tmpl$14();
          insert(_el$32, createComponent(Badge, {
            get children() {
              return `agents=${gameplay().entityCounts.agents}`;
            }
          }), null);
          insert(_el$32, createComponent(Badge, {
            get children() {
              return `locations=${gameplay().entityCounts.locations}`;
            }
          }), null);
          return _el$32;
        }
      }), (() => {
        var _el$33 = _tmpl$4();
        insert(_el$33, () => tr(locale(), "如果中间栏仍保留“刷新快照”动作，先从那里重拉一次；如果数量仍然是 0，就需要修复或重启 runtime world bootstrap。", "If the middle column still exposes a refresh action, pull a fresh snapshot there first. If the counts stay at 0, repair or restart the runtime world bootstrap."));
        return _el$33;
      })()];
    }
  });
}
function ViewerEntryMenu() {
  const locale = () => uiLocale();
  const viewerEntryUrls = () => buildViewerEntryUrls(locale());
  return (() => {
    var _el$34 = _tmpl$15(), _el$35 = _el$34.firstChild, _el$36 = _el$35.nextSibling, _el$37 = _el$36.firstChild, _el$38 = _el$37.firstChild, _el$39 = _el$38.nextSibling, _el$40 = _el$37.nextSibling, _el$41 = _el$40.firstChild, _el$42 = _el$41.nextSibling, _el$43 = _el$40.nextSibling, _el$44 = _el$43.nextSibling;
    insert(_el$35, () => tr(locale(), "入口", "Entry"));
    insert(_el$38, () => tr(locale(), "语言与 Viewer 入口", "Language and Viewer Entry"));
    insert(_el$39, () => tr(locale(), "主玩法继续留在当前页面；这里只保留语言切换。", "Primary gameplay stays on this page. This menu only keeps locale switching."));
    _el$41.$$click = () => setViewerLocale("zh");
    _el$42.$$click = () => setViewerLocale("en");
    insert(_el$43, createComponent(Badge, {
      get children() {
        return `locale=${localeCode(locale())}`;
      }
    }));
    insert(_el$44, () => viewerEntryUrls().softwareSafeUrl);
    createRenderEffect((_p$) => {
      var _v$ = locale() === "zh", _v$2 = locale() === "en";
      _v$ !== _p$.e && (_el$41.disabled = _p$.e = _v$);
      _v$2 !== _p$.t && (_el$42.disabled = _p$.t = _v$2);
      return _p$;
    }, {
      e: void 0,
      t: void 0
    });
    return _el$34;
  })();
}
function gameplayStatusBadgeClass(status) {
  return status === "blocked" ? "badge badge--warn" : status === "branch_ready" ? "badge badge--good" : "badge badge--accent";
}
function gameplayStageToneClass(status) {
  return status === "blocked" ? "hero-focus-card__value hero-focus-card__value--warn" : status === "branch_ready" ? "hero-focus-card__value hero-focus-card__value--good" : "hero-focus-card__value hero-focus-card__value--accent";
}
function gameplayStageLabel(status, locale) {
  if (status === "blocked") {
    return tr(locale, "当前受阻", "Blocked Now");
  }
  if (status === "branch_ready") {
    return tr(locale, "可以推进", "Ready to Act");
  }
  if (status === "active") {
    return tr(locale, "正在推进", "In Motion");
  }
  if (status === "completed") {
    return tr(locale, "阶段完成", "Stage Complete");
  }
  return status || tr(locale, "等待同步", "Waiting for Sync");
}
function gameplayProgressLabel(progressPercent, locale) {
  return progressPercent == null ? tr(locale, "进度待发布", "Progress Pending") : tr(locale, `进度 ${progressPercent}%`, `Progress ${progressPercent}%`);
}
function connectionStatusLabel(status, locale) {
  if (status === "connected") {
    return tr(locale, "世界在线", "World Live");
  }
  if (status === "connecting") {
    return tr(locale, "正在连入世界", "Connecting to World");
  }
  if (status === "closed") {
    return tr(locale, "连接已关闭", "Connection Closed");
  }
  return tr(locale, `连接异常：${status || "unknown"}`, `Connection Issue: ${status || "unknown"}`);
}
function renderResourceSummary(resources) {
  return resourceSummary(resources);
}
function WorldStageHero() {
  const locale = () => uiLocale();
  const gameplaySummary = () => buildGameplaySummary(locale());
  const selectedLabel = () => state.selectedKind && state.selectedId ? `${state.selectedKind}:${state.selectedId}` : tr(locale(), "尚未选择目标", "No target selected");
  const selectionHint = () => state.selectedKind && state.selectedId ? tr(locale(), "右侧命令面会围绕这个对象展开。", "The command surface on the right now follows this target.") : tr(locale(), "先从左侧锁定一个 Agent 或地点，再进入右侧命令面。", "Lock onto an agent or location from the left before entering the command surface.");
  const stageLabel = () => gameplayStageLabel(gameplaySummary()?.stageStatus, locale());
  const nextStepCopy = () => gameplaySummary()?.nextStepHint || tr(locale(), "先读世界状态，再决定是否推进、恢复或对目标发消息。", "Read the world first, then decide whether to advance, resume, or message the target.");
  return (() => {
    var _el$45 = _tmpl$16(), _el$46 = _el$45.firstChild, _el$47 = _el$46.firstChild, _el$48 = _el$47.firstChild, _el$49 = _el$48.nextSibling, _el$50 = _el$49.nextSibling, _el$51 = _el$46.nextSibling, _el$52 = _el$51.firstChild, _el$53 = _el$52.firstChild, _el$54 = _el$53.nextSibling, _el$55 = _el$54.nextSibling, _el$56 = _el$52.nextSibling, _el$57 = _el$56.firstChild, _el$58 = _el$57.nextSibling, _el$59 = _el$58.nextSibling, _el$60 = _el$56.nextSibling, _el$61 = _el$60.firstChild, _el$62 = _el$61.nextSibling;
    insert(_el$48, () => tr(locale(), "工业世界指挥桌", "Industrial World Command Desk"));
    insert(_el$49, () => gameplaySummary()?.goalTitle || tr(locale(), "进入世界，先看局势，再做动作", "Read the world first, then act."));
    insert(_el$50, () => gameplaySummary()?.nextStepHint || gameplaySummary()?.objective || tr(locale(), "这张入口页优先保留世界、目标和关键动作；高级诊断与治理能力按需展开。", "This entry keeps the world, objective, and primary actions in front. Advanced diagnostics and governance stay on demand."));
    insert(_el$46, createComponent(ViewerEntryMenu, {}), null);
    insert(_el$53, () => tr(locale(), "局势", "Situation"));
    insert(_el$54, stageLabel);
    insert(_el$55, () => gameplayProgressLabel(gameplaySummary()?.progressPercent, locale()));
    insert(_el$57, () => tr(locale(), "当前选择", "Current Selection"));
    insert(_el$58, selectedLabel);
    insert(_el$59, selectionHint);
    insert(_el$61, () => tr(locale(), "下一步", "Next Step"));
    insert(_el$62, nextStepCopy);
    insert(_el$45, createComponent(Show, {
      get when() {
        return state.connectionStatus !== "connected";
      },
      get children() {
        return createComponent(CalloutCard, {
          get title() {
            return tr(locale(), "世界连接需要注意", "World Connection Needs Attention");
          },
          get badge() {
            return connectionStatusLabel(state.connectionStatus, locale());
          },
          get badgeClass() {
            return connectionBadgeClass();
          },
          variant: "warn",
          get children() {
            var _el$63 = _tmpl$13();
            insert(_el$63, () => tr(locale(), "首屏优先展示世界与目标；只有连接异常时，才把连接状态抬到这里提示你。", "This entry keeps the world and target first, and only elevates connection status when it needs attention."));
            return _el$63;
          }
        });
      }
    }), null);
    createRenderEffect(() => className(_el$54, gameplayStageToneClass(gameplaySummary()?.stageStatus)));
    return _el$45;
  })();
}
function MobileJumpRail() {
  const locale = () => uiLocale();
  return (() => {
    var _el$64 = _tmpl$17(), _el$65 = _el$64.firstChild, _el$66 = _el$65.nextSibling, _el$67 = _el$66.nextSibling;
    insert(_el$65, () => tr(locale(), "世界", "World"));
    insert(_el$66, () => tr(locale(), "目标", "Targets"));
    insert(_el$67, () => tr(locale(), "指挥", "Command"));
    createRenderEffect(() => setAttribute(_el$64, "aria-label", tr(locale(), "主入口分区导航", "Primary entry section navigation")));
    return _el$64;
  })();
}
function TargetsPanel() {
  const lists = () => modelLists();
  const locale = () => uiLocale();
  const selectedLabel = () => state.selectedKind && state.selectedId ? `${state.selectedKind}:${state.selectedId}` : null;
  return (() => {
    var _el$68 = _tmpl$18(), _el$69 = _el$68.firstChild, _el$70 = _el$69.firstChild, _el$71 = _el$70.nextSibling, _el$72 = _el$69.nextSibling, _el$73 = _el$72.firstChild, _el$74 = _el$73.nextSibling, _el$75 = _el$72.nextSibling, _el$76 = _el$75.firstChild, _el$77 = _el$76.nextSibling;
    insert(_el$68, createComponent(Show, {
      get when() {
        return selectedLabel();
      },
      children: (selected) => (() => {
        var _el$78 = _tmpl$14();
        insert(_el$78, createComponent(Badge, {
          "class": "badge badge--accent",
          get children() {
            return tr(locale(), "已锁定目标", "Locked Target");
          }
        }), null);
        insert(_el$78, createComponent(Badge, {
          get children() {
            return selected();
          }
        }), null);
        return _el$78;
      })()
    }), _el$69);
    insert(_el$68, createComponent(EmptyState, {
      get children() {
        return tr(locale(), "先从这里锁定一个 Agent 或地点。中间读局势，右侧只处理你当前选中的目标。", "Lock onto an agent or location here first. Read the world in the middle, then use the right column only for the selected target.");
      }
    }), _el$69);
    insert(_el$70, () => tr(locale(), "筛选目标", "Filter targets"));
    _el$71.$$input = (event) => setSelectedSearch(event.currentTarget.value);
    insert(_el$73, () => tr(locale(), "Agents", "Agents"));
    insert(_el$74, createComponent(Show, {
      get when() {
        return lists().agents.length > 0;
      },
      get fallback() {
        return createComponent(EmptyState, {
          get children() {
            return tr(locale(), "当前快照里没有 Agent。", "No agents in current snapshot.");
          }
        });
      },
      get children() {
        return createComponent(For, {
          get each() {
            return lists().agents;
          },
          children: (agent) => (() => {
            var _el$79 = _tmpl$19(), _el$80 = _el$79.firstChild, _el$81 = _el$80.nextSibling;
            _el$79.$$click = () => applySelection({
              kind: "agent",
              id: agent.id
            });
            insert(_el$80, () => agent.id);
            insert(_el$81, () => `${tr(locale(), "地点", "location")}=${agent.location_id} · ${tr(locale(), "资源", "resources")}=${renderResourceSummary(agent.resources)}`);
            createRenderEffect((_p$) => {
              var _v$3 = agent.id, _v$4 = state.selectedKind === "agent" && state.selectedId === agent.id;
              _v$3 !== _p$.e && setAttribute(_el$79, "data-select-id", _p$.e = _v$3);
              _v$4 !== _p$.t && setAttribute(_el$79, "data-selected", _p$.t = _v$4);
              return _p$;
            }, {
              e: void 0,
              t: void 0
            });
            return _el$79;
          })()
        });
      }
    }));
    insert(_el$76, () => tr(locale(), "地点", "Locations"));
    insert(_el$77, createComponent(Show, {
      get when() {
        return lists().locations.length > 0;
      },
      get fallback() {
        return createComponent(EmptyState, {
          get children() {
            return tr(locale(), "当前快照里没有地点。", "No locations in current snapshot.");
          }
        });
      },
      get children() {
        return createComponent(For, {
          get each() {
            return lists().locations;
          },
          children: (location) => (() => {
            var _el$82 = _tmpl$20(), _el$83 = _el$82.firstChild, _el$84 = _el$83.nextSibling;
            _el$82.$$click = () => applySelection({
              kind: "location",
              id: location.id
            });
            insert(_el$83, () => location.name || location.id);
            insert(_el$84, () => `id=${location.id} · ${tr(locale(), "半径", "radius")}=${formatPhysicalDistanceCm(location.profile?.radius_cm, locale()) || "-"} · ${tr(locale(), "资源", "resources")}=${renderResourceSummary(location.resources)}`);
            createRenderEffect((_p$) => {
              var _v$5 = location.id, _v$6 = state.selectedKind === "location" && state.selectedId === location.id;
              _v$5 !== _p$.e && setAttribute(_el$82, "data-select-id", _p$.e = _v$5);
              _v$6 !== _p$.t && setAttribute(_el$82, "data-selected", _p$.t = _v$6);
              return _p$;
            }, {
              e: void 0,
              t: void 0
            });
            return _el$82;
          })()
        });
      }
    }));
    createRenderEffect(() => setAttribute(_el$71, "placeholder", tr(locale(), "搜索 Agent 或地点", "Search agents or locations")));
    createRenderEffect(() => _el$71.value = getSelectedSearch());
    return _el$68;
  })();
}
function WorldSummaryPanel() {
  const locale = () => uiLocale();
  const state$1 = state;
  const gameplaySummary = () => buildGameplaySummary(locale());
  const gameplayActionFeedback = () => snapshotSemanticFeedback(state$1.lastGameplayActionFeedback);
  const promptFeedback = () => snapshotSemanticFeedback(state$1.lastPromptFeedback);
  const chatFeedback = () => snapshotSemanticFeedback(state$1.lastChatFeedback);
  const gameplayActionFeedbackDisplay = () => describeSemanticFeedback(gameplayActionFeedback(), locale());
  const promptFeedbackDisplay = () => describeSemanticFeedback(promptFeedback(), locale());
  const chatFeedbackDisplay = () => describeSemanticFeedback(chatFeedback(), locale());
  const authSurface = () => buildAuthSurfaceModel();
  const hostedActionMatrixView = () => buildHostedActionMatrixView();
  const hostedRecoveryHint = () => buildHostedRecoveryHint(locale());
  const selectedDebug = () => selectedAgentExecutionDebugContext();
  const tierBadgeClass = (status) => status === "active" || status === "active_legacy_preview" ? "badge badge--good" : status === "superseded" ? "badge" : "badge badge--warn";
  const showRebindNotice = () => Boolean(state$1.auth.pendingRequestedAgentId) && (state$1.auth.pendingForceRebind || state$1.auth.runtimeStatus === "rebind_retrying" || state$1.auth.runtimeStatus === "rebind_registering");
  const showPlayerSessionSurface = () => !!hostedRecoveryHint() || !state$1.auth.available && String(state$1.hostedAccess?.deployment_mode || "").trim() === "hosted_public_join" || showRebindNotice();
  const diagnosticsSummaryBadges = () => [`debugViewer=${state$1.debugViewerMode}:${state$1.debugViewerStatus}`, `auth=${state$1.auth.available ? state$1.auth.registrationStatus || "ready" : "missing"}`, `events=${state$1.recentEvents.length}`];
  return (() => {
    var _el$85 = _tmpl$26(), _el$91 = _el$85.firstChild, _el$92 = _el$91.firstChild, _el$93 = _el$92.firstChild, _el$94 = _el$93.firstChild, _el$95 = _el$94.nextSibling, _el$96 = _el$93.nextSibling, _el$97 = _el$92.nextSibling, _el$98 = _el$97.firstChild, _el$100 = _el$98.nextSibling, _el$101 = _el$100.nextSibling, _el$110 = _el$101.nextSibling, _el$111 = _el$110.nextSibling, _el$112 = _el$111.firstChild, _el$113 = _el$112.nextSibling;
    insert(_el$85, createComponent(PanelSection, {
      get title() {
        return tr(locale(), "正式玩法摘要", "Formal Gameplay Summary");
      },
      get eyebrow() {
        return tr(locale(), "玩家主路径", "Player Path");
      },
      get meta() {
        return tr(locale(), "先看目标、阻塞和下一步，再决定是否进入右侧命令区。", "Read the goal, blocker, and next step first, then decide whether to enter the command surface.");
      },
      get children() {
        return createComponent(Show, {
          get when() {
            return gameplaySummary();
          },
          get fallback() {
            return createComponent(EmptyState, {
              get children() {
                return tr(locale(), "等待首条 canonical gameplay 快照…", "Waiting for the first canonical gameplay snapshot…");
              }
            });
          },
          children: (gameplay) => [(() => {
            var _el$114 = _tmpl$14();
            insert(_el$114, createComponent(Badge, {
              get ["class"]() {
                return gameplayStatusBadgeClass(gameplay().stageStatus);
              },
              get children() {
                return gameplayStageLabel(gameplay().stageStatus, locale());
              }
            }), null);
            insert(_el$114, createComponent(Badge, {
              "class": "badge badge--accent",
              get children() {
                return gameplayProgressLabel(gameplay().progressPercent, locale());
              }
            }), null);
            return _el$114;
          })(), createComponent(Show, {
            get when() {
              return gameplay().blockerKind || gameplay().blockerDetail;
            },
            get children() {
              return createComponent(CalloutCard, {
                get title() {
                  return memo(() => gameplay().blockerKind === "runtime_snapshot_empty_entities")() ? tr(locale(), "当前阻塞：空快照", "Current Blocker: Empty Snapshot") : tr(locale(), "当前阻塞", "Current Blocker");
                },
                get badge() {
                  return gameplay().blockerKind || "blocked";
                },
                badgeClass: "badge badge--warn",
                variant: "warn",
                get children() {
                  return [(() => {
                    var _el$115 = _tmpl$13();
                    insert(_el$115, () => gameplay().blockerDetail || tr(locale(), "当前玩法被阻塞，需要显式恢复。", "Gameplay is blocked and needs explicit recovery."));
                    return _el$115;
                  })(), createComponent(Show, {
                    get when() {
                      return gameplay().blockerSupplementalDetail;
                    },
                    get children() {
                      var _el$116 = _tmpl$4();
                      insert(_el$116, () => gameplay().blockerSupplementalDetail);
                      return _el$116;
                    }
                  }), createComponent(Show, {
                    get when() {
                      return gameplay().nextStepHint;
                    },
                    get children() {
                      var _el$117 = _tmpl$4();
                      insert(_el$117, () => gameplay().nextStepHint);
                      return _el$117;
                    }
                  }), createComponent(Show, {
                    get when() {
                      return gameplay().entityCounts;
                    },
                    get children() {
                      var _el$118 = _tmpl$14();
                      insert(_el$118, createComponent(Badge, {
                        get children() {
                          return `agents=${gameplay().entityCounts.agents}`;
                        }
                      }), null);
                      insert(_el$118, createComponent(Badge, {
                        get children() {
                          return `locations=${gameplay().entityCounts.locations}`;
                        }
                      }), null);
                      return _el$118;
                    }
                  })];
                }
              });
            }
          }), createComponent(EventCard, {
            get title() {
              return gameplay().goalTitle || tr(locale(), "当前目标", "Current Goal");
            },
            get badge() {
              return memo(() => gameplay().progressPercent == null)() ? "n/a" : `${gameplay().progressPercent}%`;
            },
            badgeClass: "badge badge--accent",
            get meta() {
              return gameplay().objective || tr(locale(), "当前还没有目标说明。", "No objective text yet.");
            },
            get children() {
              return createComponent(Show, {
                get when() {
                  return gameplay().progressDetail;
                },
                get children() {
                  var _el$119 = _tmpl$4();
                  insert(_el$119, () => gameplay().progressDetail);
                  return _el$119;
                }
              });
            }
          }), createComponent(Show, {
            get when() {
              return gameplay().recentFeedback;
            },
            children: (feedback) => createComponent(EventCard, {
              get title() {
                return tr(locale(), "最近玩法反馈", "Recent Gameplay Feedback");
              },
              get badge() {
                return feedback().stage || "-";
              },
              get badgeClass() {
                return feedback().stage === "blocked" ? "badge badge--warn" : "badge badge--good";
              },
              get meta() {
                return memo(() => !!feedback().action)() ? tr(locale(), `来自动作 ${feedback().action}`, `From action ${feedback().action}`) : tr(locale(), "最近一条玩法回执", "Most recent gameplay feedback");
              },
              get children() {
                return [(() => {
                  var _el$127 = _tmpl$13();
                  insert(_el$127, () => feedback().effect || feedback().reason || "Gameplay feedback updated.");
                  return _el$127;
                })(), createComponent(Show, {
                  get when() {
                    return feedback().reason;
                  },
                  get children() {
                    var _el$128 = _tmpl$4();
                    insert(_el$128, () => feedback().reason);
                    return _el$128;
                  }
                }), createComponent(Show, {
                  get when() {
                    return feedback().hint;
                  },
                  get children() {
                    var _el$129 = _tmpl$4();
                    insert(_el$129, () => feedback().hint);
                    return _el$129;
                  }
                })];
              }
            })
          }), createComponent(EventCard, {
            get title() {
              return tr(locale(), "下一步", "Next Step");
            },
            get badge() {
              return gameplay().stageStatus || "-";
            },
            get children() {
              return [(() => {
                var _el$120 = _tmpl$13();
                insert(_el$120, () => gameplay().nextStepHint || tr(locale(), "等待下一次 runtime 指引更新。", "Wait for the next runtime guidance update."));
                return _el$120;
              })(), createComponent(Show, {
                get when() {
                  return gameplay().branchHint;
                },
                get children() {
                  var _el$121 = _tmpl$4();
                  insert(_el$121, () => gameplay().branchHint);
                  return _el$121;
                }
              })];
            }
          }), createComponent(Show, {
            get when() {
              return gameplayActionFeedback();
            },
            children: (feedback) => createComponent(FeedbackCard, {
              get feedback() {
                return feedback();
              },
              get display() {
                return gameplayActionFeedbackDisplay();
              }
            })
          }), (() => {
            var _el$122 = _tmpl$27(), _el$123 = _el$122.firstChild, _el$124 = _el$123.nextSibling;
            insert(_el$123, () => tr(locale(), "可用玩法动作", "Available Gameplay Actions"));
            insert(_el$124, createComponent(Show, {
              get when() {
                return gameplay().availableActions.length > 0;
              },
              get fallback() {
                return createComponent(EmptyState, {
                  get children() {
                    return tr(locale(), "当前还没有发布 canonical gameplay 动作。", "No canonical gameplay actions published yet.");
                  }
                });
              },
              get children() {
                return createComponent(For, {
                  get each() {
                    return gameplay().availableActions;
                  },
                  children: (action) => createComponent(EventCard, {
                    "class": "event-card event-card--action",
                    get title() {
                      return action.label || action.actionId || "unknown_action";
                    },
                    get badge() {
                      return action.disabledReason ? "handoff" : "ready";
                    },
                    get badgeClass() {
                      return action.disabledReason ? "badge badge--warn" : "badge badge--good";
                    },
                    get meta() {
                      return memo(() => !!action.targetAgentId)() ? tr(locale(), `作用对象 ${action.targetAgentId}`, `Acts on ${action.targetAgentId}`) : tr(locale(), "世界级动作", "World-level action");
                    },
                    get children() {
                      return [(() => {
                        var _el$130 = _tmpl$4();
                        insert(_el$130, () => action.disabledReason || tr(locale(), "无需打开 visual QA viewer，也可以直接从正式 Web 入口执行。", "Playable from the formal Web entry without opening the visual QA viewer."));
                        return _el$130;
                      })(), createComponent(Show, {
                        get when() {
                          return action.executeKind === "request_snapshot" || action.executeKind === "step" || action.executeKind === "play" || action.executeKind === "gameplay_action";
                        },
                        get children() {
                          var _el$131 = _tmpl$28(), _el$132 = _el$131.firstChild;
                          _el$132.$$click = () => sendGameplayAction(action);
                          insert(_el$132, (() => {
                            var _c$ = memo(() => action.executeKind === "request_snapshot");
                            return () => _c$() ? tr(locale(), "刷新快照", "Refresh Snapshot") : memo(() => action.executeKind === "step")() ? tr(locale(), "推进一步", "Advance One Step") : memo(() => action.executeKind === "play")() ? tr(locale(), "恢复实时推进", "Resume Live Play") : tr(locale(), "提交玩法动作", "Submit Gameplay Action");
                          })());
                          createRenderEffect(() => _el$132.disabled = Boolean(action.disabledReason));
                          return _el$131;
                        }
                      }), createComponent(Show, {
                        get when() {
                          return action.executeKind === "agent_chat";
                        },
                        get children() {
                          var _el$133 = _tmpl$28(), _el$134 = _el$133.firstChild;
                          _el$134.$$click = () => applySelection({
                            kind: "agent",
                            id: action.targetAgentId
                          });
                          insert(_el$134, () => tr(locale(), "切到聊天面板", "Use Chat Panel"));
                          createRenderEffect(() => _el$134.disabled = Boolean(action.disabledReason));
                          return _el$133;
                        }
                      })];
                    }
                  })
                });
              }
            }));
            return _el$122;
          })(), createComponent(CalloutCard, {
            get title() {
              return tr(locale(), "未在此页暴露的动作", "Actions Not Exposed On This Page");
            },
            badge: "handoff",
            badgeClass: "badge badge--warn",
            get children() {
              return [(() => {
                var _el$125 = _tmpl$13();
                insert(_el$125, () => gameplay().assetGovernanceHandoff);
                return _el$125;
              })(), (() => {
                var _el$126 = _tmpl$4();
                insert(_el$126, () => tr(locale(), "资产 / 治理相关能力请走单独 lane；这张主入口页面只保留正式玩法所需的最小动作面。", "Asset and governance actions stay on their dedicated lane; this primary entry only keeps the minimum surface needed for formal gameplay."));
                return _el$126;
              })()];
            }
          })]
        });
      }
    }), _el$91);
    insert(_el$85, createComponent(Show, {
      get when() {
        return showPlayerSessionSurface();
      },
      get children() {
        return createComponent(PanelSection, {
          get title() {
            return tr(locale(), "进入会话", "Player Access");
          },
          get eyebrow() {
            return tr(locale(), "只在需要时出现", "Only When Needed");
          },
          get meta() {
            return tr(locale(), "只有当玩家会话缺失、重绑中或需要恢复时，这里才会打断主玩法路径。", "This only interrupts the main path when the player session is missing, rebinding, or needs recovery.");
          },
          get children() {
            return [(() => {
              var _el$86 = _tmpl$14();
              insert(_el$86, createComponent(Badge, {
                get ["class"]() {
                  return state$1.auth.available ? "badge badge--good" : "badge badge--warn";
                },
                get children() {
                  return `auth=${state$1.auth.available ? state$1.auth.registrationStatus || "ready" : "missing"}`;
                }
              }), null);
              insert(_el$86, createComponent(Badge, {
                "class": "badge badge--accent",
                get children() {
                  return `tier=${authSurface().currentTier}`;
                }
              }), null);
              insert(_el$86, createComponent(Badge, {
                get children() {
                  return `player=${state$1.auth.playerId || "-"}`;
                }
              }), null);
              insert(_el$86, createComponent(Badge, {
                get children() {
                  return `boundAgent=${state$1.auth.boundAgentId || "-"}`;
                }
              }), null);
              return _el$86;
            })(), createComponent(EmptyState, {
              get children() {
                return hostedRecoveryHint()?.detail || state$1.auth.rebindNotice || authSurface().currentTierReason;
              }
            }), createComponent(Show, {
              get when() {
                return hostedRecoveryHint();
              },
              children: (hint) => (() => {
                var _el$135 = _tmpl$21(), _el$136 = _el$135.firstChild;
                _el$136.$$click = () => {
                  void retryHostedPlayerIdentityIssue();
                };
                insert(_el$136, () => hint().cta);
                createRenderEffect(() => _el$136.disabled = state$1.auth.issueInFlight);
                return _el$135;
              })()
            }), createComponent(Show, {
              get when() {
                return memo(() => !!(!state$1.auth.available && String(state$1.hostedAccess?.deployment_mode || "").trim() === "hosted_public_join"))() && !hostedRecoveryHint();
              },
              get children() {
                var _el$87 = _tmpl$21(), _el$88 = _el$87.firstChild;
                _el$88.$$click = () => {
                  void retryHostedPlayerIdentityIssue();
                };
                insert(_el$88, () => tr(locale(), "领取玩家会话", "Acquire Player Session"));
                createRenderEffect(() => _el$88.disabled = state$1.auth.issueInFlight);
                return _el$87;
              }
            }), createComponent(Show, {
              get when() {
                return memo(() => !!state$1.auth.available)() && state$1.auth.source !== "legacy_viewer_auth_bootstrap";
              },
              get children() {
                var _el$89 = _tmpl$22(), _el$90 = _el$89.firstChild;
                _el$90.$$click = () => {
                  void logoutHostedPlayerSession();
                };
                insert(_el$90, () => tr(locale(), "释放玩家会话", "Release Player Session"));
                return _el$89;
              }
            })];
          }
        });
      }
    }), _el$91);
    insert(_el$94, () => tr(locale(), "运行诊断", "Runtime Diagnostics"));
    insert(_el$95, () => tr(locale(), "执行 lane、auth/session、托管矩阵与最近事件都收在这里，避免它们继续抢占主玩法首屏。", "Execution lanes, auth/session truth, hosted matrix, and recent events live here so they no longer dominate the primary gameplay viewport."));
    insert(_el$96, createComponent(For, {
      get each() {
        return diagnosticsSummaryBadges();
      },
      children: (label) => createComponent(Badge, {
        children: label
      })
    }));
    insert(_el$98, createComponent(Badge, {
      get children() {
        return `ws=${state$1.wsUrl || "-"}`;
      }
    }), null);
    insert(_el$98, createComponent(Badge, {
      get children() {
        return `entryReason=${state$1.viewerReason || "-"}`;
      }
    }), null);
    insert(_el$98, createComponent(Badge, {
      get children() {
        return `renderer=${state$1.renderer || "n/a"}`;
      }
    }), null);
    insert(_el$98, createComponent(Badge, {
      get children() {
        return `controlProfile=${state$1.controlProfile}`;
      }
    }), null);
    insert(_el$97, createComponent(PanelSection, {
      get title() {
        return tr(locale(), "执行 Lane", "Execution Lanes");
      },
      get children() {
        return [(() => {
          var _el$99 = _tmpl$14();
          insert(_el$99, createComponent(Badge, {
            "class": "badge badge--accent",
            children: "debug_viewer"
          }), null);
          insert(_el$99, createComponent(Badge, {
            get children() {
              return `status=${state$1.debugViewerStatus}`;
            }
          }), null);
          insert(_el$99, createComponent(Badge, {
            get children() {
              return `renderMode=${state$1.renderMode}`;
            }
          }), null);
          insert(_el$99, createComponent(Badge, {
            get children() {
              return `entryReason=${state$1.viewerReason || "-"}`;
            }
          }), null);
          return _el$99;
        })(), createComponent(EmptyState, {
          style: "margin-top:-2px;",
          get children() {
            return tr(locale(), "debug_viewer 是只读订阅 lane，只负责消费 runtime 快照和事件；关闭这个 viewer 不会停止 agent lane。", "debug_viewer is a read-only subscription lane for runtime snapshots/events; closing the viewer does not stop the agent lane.");
          }
        }), createComponent(Show, {
          get when() {
            return selectedDebug();
          },
          get fallback() {
            return createComponent(EmptyState, {
              children: "Select an agent to compare the headless execution lane against this debug_viewer observer lane."
            });
          },
          children: (debug) => [(() => {
            var _el$137 = _tmpl$14();
            insert(_el$137, createComponent(Badge, {
              "class": "badge badge--accent",
              children: "selected agent lane"
            }), null);
            insert(_el$137, createComponent(Badge, {
              get children() {
                return `provider=${debug().provider_mode || "-"}`;
              }
            }), null);
            insert(_el$137, createComponent(Badge, {
              get children() {
                return `mode=${debug().execution_mode || "-"}`;
              }
            }), null);
            insert(_el$137, createComponent(Badge, {
              get children() {
                return `env=${debug().environment_class || "-"}`;
              }
            }), null);
            return _el$137;
          })(), (() => {
            var _el$138 = _tmpl$14();
            insert(_el$138, createComponent(Badge, {
              get children() {
                return `obs=${debug().observation_schema_version || "-"}`;
              }
            }), null);
            insert(_el$138, createComponent(Badge, {
              get children() {
                return `act=${debug().action_schema_version || "-"}`;
              }
            }), null);
            insert(_el$138, createComponent(Badge, {
              get children() {
                return `agentProfile=${debug().agent_profile || "-"}`;
              }
            }), null);
            insert(_el$138, createComponent(Badge, {
              get children() {
                return `providerFallback=${debug().fallback_reason || "-"}`;
              }
            }), null);
            return _el$138;
          })(), createComponent(EmptyState, {
            style: "margin-top:-2px;",
            get children() {
              return tr(locale(), "上面的 lane badge 表示 phase-1 期望执行 contract；下面的 provider check badge 表示 runtime_live 基于 /v1/provider/info 和 /v1/provider/health 的真实探测结果。", "Lane badges show the expected phase-1 execution contract. Provider check badges below show the actual runtime_live probe against /v1/provider/info and /v1/provider/health.");
            }
          }), (() => {
            var _el$139 = _tmpl$14();
            insert(_el$139, createComponent(Badge, {
              "class": "badge badge--accent",
              children: "provider check"
            }), null);
            insert(_el$139, createComponent(Badge, {
              get children() {
                return `status=${debug().provider_check_status || "-"}`;
              }
            }), null);
            insert(_el$139, createComponent(Badge, {
              get children() {
                return `source=${debug().provider_check_source || "-"}`;
              }
            }), null);
            insert(_el$139, createComponent(Badge, {
              get children() {
                return `fallback=${debug().provider_check_fallback_reason || "-"}`;
              }
            }), null);
            return _el$139;
          })(), createComponent(Show, {
            get when() {
              return debug().provider_check_error || debug().provider_reported_capabilities?.length || debug().provider_reported_supported_action_sets?.length;
            },
            get children() {
              var _el$140 = _tmpl$14();
              insert(_el$140, createComponent(Badge, {
                get children() {
                  return `actualCaps=${(debug().provider_reported_capabilities || []).join(",") || "-"}`;
                }
              }), null);
              insert(_el$140, createComponent(Badge, {
                get children() {
                  return `actualActions=${(debug().provider_reported_supported_action_sets || []).join(",") || "-"}`;
                }
              }), null);
              insert(_el$140, createComponent(Badge, {
                get children() {
                  return `checkError=${debug().provider_check_error || "-"}`;
                }
              }), null);
              return _el$140;
            }
          }), createComponent(JsonBlock, {
            get value() {
              return debug();
            }
          })]
        })];
      }
    }), _el$100);
    insert(_el$100, createComponent(Badge, {
      get ["class"]() {
        return state$1.auth.available ? "badge badge--good" : "badge badge--warn";
      },
      get children() {
        return `auth=${state$1.auth.available ? state$1.auth.registrationStatus || "ready" : "missing"}`;
      }
    }), null);
    insert(_el$100, createComponent(Badge, {
      "class": "badge badge--accent",
      get children() {
        return `tier=${authSurface().currentTier}`;
      }
    }), null);
    insert(_el$100, createComponent(Badge, {
      get children() {
        return `source=${authSurface().source}`;
      }
    }), null);
    insert(_el$100, createComponent(Badge, {
      get children() {
        return `deploymentHint=${authSurface().deploymentHint}`;
      }
    }), null);
    insert(_el$100, createComponent(Badge, {
      get children() {
        return `player=${state$1.auth.playerId || "-"}`;
      }
    }), null);
    insert(_el$100, createComponent(Badge, {
      get children() {
        return `pubkey=${state$1.auth.publicKey ? `${state$1.auth.publicKey.slice(0, 10)}…` : "-"}`;
      }
    }), null);
    insert(_el$100, createComponent(Badge, {
      get children() {
        return `epoch=${state$1.auth.sessionEpoch == null ? "-" : state$1.auth.sessionEpoch}`;
      }
    }), null);
    insert(_el$100, createComponent(Badge, {
      get children() {
        return `runtime=${state$1.auth.runtimeStatus || "-"}`;
      }
    }), null);
    insert(_el$100, createComponent(Badge, {
      get children() {
        return `boundAgent=${state$1.auth.boundAgentId || "-"}`;
      }
    }), null);
    insert(_el$100, createComponent(Badge, {
      get children() {
        return `requestedAgent=${state$1.auth.pendingRequestedAgentId || "-"}`;
      }
    }), null);
    insert(_el$100, createComponent(Badge, {
      get children() {
        return state$1.auth.pendingForceRebind ? "rebind=forcing" : "rebind=idle";
      }
    }), null);
    insert(_el$101, createComponent(Show, {
      get when() {
        return hostedRecoveryHint();
      },
      children: (hint) => (() => {
        var _el$141 = _tmpl$23();
        _el$141.$$click = () => {
          void retryHostedPlayerIdentityIssue();
        };
        insert(_el$141, () => hint().cta);
        createRenderEffect(() => _el$141.disabled = state$1.auth.issueInFlight);
        return _el$141;
      })()
    }), null);
    insert(_el$101, createComponent(Show, {
      get when() {
        return memo(() => !!(!state$1.auth.available && String(state$1.hostedAccess?.deployment_mode || "").trim() === "hosted_public_join"))() && !hostedRecoveryHint();
      },
      get children() {
        var _el$102 = _tmpl$23();
        _el$102.$$click = () => {
          void retryHostedPlayerIdentityIssue();
        };
        insert(_el$102, () => tr(locale(), "领取玩家会话", "Acquire Player Session"));
        createRenderEffect(() => _el$102.disabled = state$1.auth.issueInFlight);
        return _el$102;
      }
    }), null);
    insert(_el$101, createComponent(Show, {
      get when() {
        return memo(() => !!state$1.auth.available)() && state$1.auth.source !== "legacy_viewer_auth_bootstrap";
      },
      get children() {
        var _el$103 = _tmpl$24();
        _el$103.$$click = () => {
          void logoutHostedPlayerSession();
        };
        insert(_el$103, () => tr(locale(), "释放玩家会话", "Release Player Session"));
        return _el$103;
      }
    }), null);
    insert(_el$97, createComponent(Show, {
      get when() {
        return state$1.auth.recoveryErrorCode || state$1.auth.recoveryErrorMessage;
      },
      get children() {
        var _el$104 = _tmpl$14();
        insert(_el$104, createComponent(Badge, {
          "class": "badge badge--warn",
          get children() {
            return `recoveryError=${state$1.auth.recoveryErrorCode || "-"}`;
          }
        }), null);
        insert(_el$104, createComponent(Badge, {
          get children() {
            return state$1.auth.recoveryErrorMessage || "-";
          }
        }), null);
        return _el$104;
      }
    }), _el$110);
    insert(_el$97, createComponent(Show, {
      get when() {
        return showRebindNotice();
      },
      get children() {
        return [(() => {
          var _el$105 = _tmpl$14();
          insert(_el$105, createComponent(Badge, {
            "class": "badge badge--accent",
            children: "rebind"
          }), null);
          insert(_el$105, createComponent(Badge, {
            get children() {
              return `target=${state$1.auth.pendingRequestedAgentId || "-"}`;
            }
          }), null);
          insert(_el$105, createComponent(Badge, {
            get children() {
              return state$1.auth.pendingForceRebind ? "mode=force_rebind" : "mode=awaiting_retry";
            }
          }), null);
          return _el$105;
        })(), createComponent(EmptyState, {
          children: "Player session is switching to the requested agent and the current action will continue after registration succeeds."
        })];
      }
    }), _el$110);
    insert(_el$97, createComponent(Show, {
      get when() {
        return state$1.hostedAdmission;
      },
      children: (admission) => (() => {
        var _el$142 = _tmpl$14();
        insert(_el$142, createComponent(Badge, {
          get children() {
            return `activeSlots=${admission().active_player_sessions}/${admission().max_player_sessions}`;
          }
        }), null);
        insert(_el$142, createComponent(Badge, {
          get children() {
            return `effectiveSlots=${admission().effective_player_sessions == null ? "-" : `${admission().effective_player_sessions}/${admission().max_player_sessions}`}`;
          }
        }), null);
        insert(_el$142, createComponent(Badge, {
          get children() {
            return `runtimeBound=${admission().runtime_bound_player_sessions ?? "-"}`;
          }
        }), null);
        insert(_el$142, createComponent(Badge, {
          get children() {
            return `runtimeOnly=${admission().runtime_only_player_sessions ?? "-"}`;
          }
        }), null);
        insert(_el$142, createComponent(Badge, {
          get children() {
            return `runtimeProbe=${admission().runtime_probe_status || "-"}`;
          }
        }), null);
        insert(_el$142, createComponent(Badge, {
          get children() {
            return `issueBudget=${admission().remaining_issue_budget}`;
          }
        }), null);
        insert(_el$142, createComponent(Badge, {
          get children() {
            return `leaseTTL=${admission().slot_lease_ttl_ms}`;
          }
        }), null);
        insert(_el$142, createComponent(Badge, {
          get children() {
            return `issued=${admission().issued_players_total}`;
          }
        }), null);
        insert(_el$142, createComponent(Badge, {
          get children() {
            return `released=${admission().released_players_total}`;
          }
        }), null);
        return _el$142;
      })()
    }), _el$110);
    insert(_el$97, createComponent(Show, {
      get when() {
        return state$1.hostedAdmission?.runtime_probe_error;
      },
      get children() {
        var _el$106 = _tmpl$14();
        insert(_el$106, createComponent(Badge, {
          "class": "badge badge--warn",
          get children() {
            return `runtimeProbeError=${state$1.hostedAdmission.runtime_probe_error}`;
          }
        }));
        return _el$106;
      }
    }), _el$110);
    insert(_el$97, createComponent(PanelSection, {
      title: "Session Ladder",
      get children() {
        return [createComponent(EmptyState, {
          get children() {
            return authSurface().currentTierReason;
          }
        }), (() => {
          var _el$107 = _tmpl$25();
          insert(_el$107, createComponent(For, {
            get each() {
              return authSurface().tiers;
            },
            children: (tier) => createComponent(EventCard, {
              get title() {
                return tier.label;
              },
              get badge() {
                return tier.status;
              },
              get badgeClass() {
                return tierBadgeClass(tier.status);
              },
              get meta() {
                return tier.reason;
              }
            })
          }));
          return _el$107;
        })(), (() => {
          var _el$108 = _tmpl$14();
          insert(_el$108, createComponent(Badge, {
            get ["class"]() {
              return authSurface().capabilities.prompt_control.enabled ? "badge badge--good" : "badge badge--warn";
            },
            get children() {
              return `prompt=${authSurface().capabilities.prompt_control.enabled ? "enabled" : authSurface().capabilities.prompt_control.code}`;
            }
          }), null);
          insert(_el$108, createComponent(Badge, {
            get ["class"]() {
              return authSurface().capabilities.agent_chat.enabled ? "badge badge--good" : "badge badge--warn";
            },
            get children() {
              return `chat=${authSurface().capabilities.agent_chat.enabled ? "enabled" : authSurface().capabilities.agent_chat.code}`;
            }
          }), null);
          insert(_el$108, createComponent(Badge, {
            "class": "badge badge--warn",
            get children() {
              return `mainToken=${authSurface().capabilities.main_token_transfer.code}`;
            }
          }), null);
          return _el$108;
        })(), createComponent(EmptyState, {
          get children() {
            return authSurface().reconnect;
          }
        })];
      }
    }), _el$110);
    insert(_el$97, createComponent(Show, {
      get when() {
        return hostedActionMatrixView().length > 0;
      },
      get children() {
        return createComponent(PanelSection, {
          get title() {
            return tr(locale(), "托管动作矩阵", "Hosted Action Matrix");
          },
          get children() {
            return [createComponent(EmptyState, {
              get children() {
                return tr(locale(), "这里是 launcher 导出的 hosted public-join 真值面。QA 应该直接读取这些 action id，而不是只靠按钮状态推断。", "This is the hosted public-join truth surface exported by the launcher. QA should read these action ids directly instead of inferring from button state alone.");
              }
            }), (() => {
              var _el$109 = _tmpl$25();
              insert(_el$109, createComponent(For, {
                get each() {
                  return hostedActionMatrixView();
                },
                children: (item) => createComponent(EventCard, {
                  get title() {
                    return item.actionId;
                  },
                  get badge() {
                    return memo(() => !!item.enabled)() ? "enabled" : item.code || "blocked";
                  },
                  get badgeClass() {
                    return item.enabled ? "badge badge--good" : "badge badge--warn";
                  },
                  get meta() {
                    return `required_auth=${item.requiredAuth} · availability=${item.availability}`;
                  },
                  get children() {
                    return [createComponent(EmptyState, {
                      get children() {
                        return item.reason || "-";
                      }
                    }), createComponent(Show, {
                      get when() {
                        return memo(() => !!item.capabilityReason)() && item.capabilityReason !== item.reason;
                      },
                      get children() {
                        return createComponent(EmptyState, {
                          get children() {
                            return `viewer=${item.capabilityReason}`;
                          }
                        });
                      }
                    })];
                  }
                })
              }));
              return _el$109;
            })()];
          }
        });
      }
    }), _el$110);
    insert(_el$110, createComponent(MetricCard, {
      get label() {
        return tr(locale(), "Prompt 反馈", "Prompt Feedback");
      },
      get value() {
        return promptFeedback()?.stage || "idle";
      },
      get children() {
        return createComponent(Show, {
          get when() {
            return promptFeedbackDisplay();
          },
          get children() {
            return createComponent(Badge, {
              get ["class"]() {
                return promptFeedbackDisplay().badgeClass;
              },
              get children() {
                return promptFeedbackDisplay().label;
              }
            });
          }
        });
      }
    }), null);
    insert(_el$110, createComponent(MetricCard, {
      get label() {
        return tr(locale(), "聊天反馈", "Chat Feedback");
      },
      get value() {
        return chatFeedback()?.stage || "idle";
      },
      get children() {
        return createComponent(Show, {
          get when() {
            return chatFeedbackDisplay();
          },
          get children() {
            return createComponent(Badge, {
              get ["class"]() {
                return chatFeedbackDisplay().badgeClass;
              },
              get children() {
                return chatFeedbackDisplay().label;
              }
            });
          }
        });
      }
    }), null);
    insert(_el$112, () => tr(locale(), "最近事件", "Recent Events"));
    insert(_el$113, createComponent(Show, {
      get when() {
        return state$1.recentEvents.length > 0;
      },
      get fallback() {
        return createComponent(EmptyState, {
          get children() {
            return tr(locale(), "等待 live 事件…", "Waiting for live events…");
          }
        });
      },
      get children() {
        return createComponent(For, {
          get each() {
            return state$1.recentEvents;
          },
          children: (event) => createComponent(EventCard, {
            get title() {
              return summarizeEventTitle(event);
            },
            get badge() {
              return `#${Number(event.id || 0)}`;
            },
            get meta() {
              return `time=${Number(event.time || 0)}`;
            },
            get children() {
              return createComponent(JsonBlock, {
                get value() {
                  return event.kind;
                }
              });
            }
          })
        });
      }
    }));
    return _el$85;
  })();
}
function InteractionPanel() {
  const locale = () => uiLocale();
  const agentId = () => selectedAgentId();
  const gameplaySummary = () => buildGameplaySummary(locale());
  const authSurface = () => buildAuthSurfaceModel();
  const promptCapability = () => authSurface().capabilities.prompt_control;
  const chatCapability = () => authSurface().capabilities.agent_chat;
  const mainTokenTransferCapability = () => authSurface().capabilities.main_token_transfer;
  const mainTokenTransferPolicy = () => hostedActionPolicy("main_token_transfer");
  const binding = () => selectedAgentBindingInfo();
  const debugContext = () => selectedAgentExecutionDebugContext();
  const promptFeedback = () => snapshotSemanticFeedback(state.lastPromptFeedback);
  const chatFeedback = () => snapshotSemanticFeedback(state.lastChatFeedback);
  const promptFeedbackDisplay = () => describeSemanticFeedback(promptFeedback(), locale());
  const chatFeedbackDisplay = () => describeSemanticFeedback(chatFeedback(), locale());
  const promptVersionState = () => describePromptVersionState(promptFeedback(), locale());
  const chatHistory = () => state.chatHistory.filter((entry) => entry.agentId === agentId() || entry.targetAgentId === agentId()).slice(0, 12);
  const interactionEnabled = () => promptCapability().enabled;
  const promptOverridesVisible = () => !!state.promptOverridesVisible;
  const assetLaneStatusText = () => mainTokenTransferCapability().enabled ? tr(locale(), "仅预览", "preview_only") : mainTokenTransferCapability().code || "blocked";
  const assetLaneDetail = () => mainTokenTransferCapability().enabled ? tr(locale(), "contract 表明这个 lane 具备 strong_auth 级 main_token_transfer 能力，但 viewer 这里仍然不会直接暴露转账表单。", "Contract marks main_token_transfer as strong_auth-capable on this lane, but viewer still exposes no transfer form here.") : mainTokenTransferCapability().reason;
  const promptSettingsSummary = () => promptOverridesVisible() ? tr(locale(), "高级 Prompt 设置已展开；你可以继续做 preview/apply/rollback，页面也会显示最近一次反馈。", "Advanced prompt settings are expanded; preview/apply/rollback and the latest prompt feedback are visible.") : tr(locale(), "Prompt Overrides 默认收起，避免把 operator 级编辑控件直接堆在主入口。显式展开后仍可做 preview/apply/rollback，`__AW_TEST__.sendPromptControl(...)` 也保持可用。", "Prompt Overrides stay hidden by default so operator-level editing controls do not dominate the primary entry. Expanding them keeps preview/apply/rollback available, and `__AW_TEST__.sendPromptControl(...)` remains available.");
  const promptSettingsButtonLabel = () => promptOverridesVisible() ? tr(locale(), "收起 Prompt Overrides", "Hide Prompt Overrides") : tr(locale(), "显示 Prompt Overrides", "Show Prompt Overrides");
  if (!agentId()) {
    if (gameplaySummary()?.blockerKind === "runtime_snapshot_empty_entities") {
      return createComponent(EmptyEntityRecoveryCard, {
        get locale() {
          return locale();
        },
        gameplay: gameplaySummary
      });
    }
    return createComponent(EmptyState, {
      get children() {
        return tr(locale(), "先选中一个 Agent，才能解锁 prompt/chat 控制。", "Select an agent to unlock prompt/chat controls.");
      }
    });
  }
  return (() => {
    var _el$143 = _tmpl$40(), _el$144 = _el$143.firstChild, _el$146 = _el$144.nextSibling;
    insert(_el$144, createComponent(Badge, {
      "class": "badge badge--accent",
      get children() {
        return tr(locale(), "当前交互目标", "Current Target");
      }
    }), null);
    insert(_el$144, createComponent(Badge, {
      get children() {
        return `agent=${agentId()}`;
      }
    }), null);
    insert(_el$144, createComponent(Badge, {
      get ["class"]() {
        return chatCapability().enabled ? "badge badge--good" : "badge badge--warn";
      },
      get children() {
        return memo(() => !!chatCapability().enabled)() ? tr(locale(), "聊天可用", "Chat Ready") : tr(locale(), "聊天受限", "Chat Limited");
      }
    }), null);
    insert(_el$143, createComponent(Show, {
      get when() {
        return debugContext()?.provider_mode === "provider_loopback_http";
      },
      get children() {
        return createComponent(EmptyState, {
          get children() {
            return tr(locale(), `当前选中的 Agent 正通过 provider-backed loopback bridge 运行在 ${debugContext()?.execution_mode || "headless_agent"}；viewer 仍处于 debug_viewer 只读观察模式，所以这里会刻意禁用 prompt/chat。`, `Selected agent currently runs through the provider-backed loopback bridge in ${debugContext()?.execution_mode || "headless_agent"}; viewer stays in debug_viewer observer-only mode, so prompt/chat are intentionally disabled here.`);
          }
        });
      }
    }), _el$146);
    insert(_el$143, createComponent(Show, {
      get when() {
        return debugContext()?.provider_mode !== "provider_loopback_http";
      },
      get children() {
        return createComponent(Show, {
          get when() {
            return interactionEnabled();
          },
          get fallback() {
            return createComponent(EmptyState, {
              get children() {
                return promptCapability().reason;
              }
            });
          },
          get children() {
            return [(() => {
              var _el$145 = _tmpl$14();
              insert(_el$145, createComponent(Badge, {
                "class": "badge badge--good",
                get children() {
                  return authSurface().currentTier;
                }
              }), null);
              insert(_el$145, createComponent(Badge, {
                get children() {
                  return `player=${state.auth.playerId}`;
                }
              }), null);
              insert(_el$145, createComponent(Badge, {
                get children() {
                  return `source=${authSurface().source}`;
                }
              }), null);
              return _el$145;
            })(), createComponent(EmptyState, {
              get children() {
                return promptCapability().reason;
              }
            })];
          }
        });
      }
    }), _el$146);
    insert(_el$146, createComponent(Badge, {
      get children() {
        return `boundPlayer=${binding()?.playerId || "-"}`;
      }
    }), null);
    insert(_el$146, createComponent(Badge, {
      get children() {
        return `boundKey=${binding()?.publicKey ? `${binding().publicKey.slice(0, 10)}…` : "-"}`;
      }
    }), null);
    insert(_el$146, createComponent(Badge, {
      get ["class"]() {
        return promptCapability().enabled ? "badge badge--good" : "badge badge--warn";
      },
      get children() {
        return `prompt=${promptCapability().enabled ? "enabled" : promptCapability().code}`;
      }
    }), null);
    insert(_el$146, createComponent(Badge, {
      get ["class"]() {
        return chatCapability().enabled ? "badge badge--good" : "badge badge--warn";
      },
      get children() {
        return `chat=${chatCapability().enabled ? "enabled" : chatCapability().code}`;
      }
    }), null);
    insert(_el$146, createComponent(Badge, {
      get ["class"]() {
        return mainTokenTransferCapability().enabled ? "badge badge--good" : "badge badge--warn";
      },
      get children() {
        return `mainToken=${assetLaneStatusText()}`;
      }
    }), null);
    insert(_el$143, createComponent(EmptyState, {
      get children() {
        return assetLaneDetail();
      }
    }), null);
    insert(_el$143, createComponent(PanelSection, {
      get title() {
        return tr(locale(), "Agent 聊天", "Agent Chat");
      },
      get eyebrow() {
        return tr(locale(), "命令面", "Command Surface");
      },
      get meta() {
        return tr(locale(), "主舞台负责看局势；这里负责向当前目标发消息和读回复。", "The stage is for reading the situation. This surface is for messaging the current target and reading replies.");
      },
      get children() {
        return [(() => {
          var _el$147 = _tmpl$29(), _el$148 = _el$147.firstChild, _el$149 = _el$148.nextSibling;
          insert(_el$148, () => tr(locale(), "消息", "Message"));
          _el$149.$$input = (event) => {
            state.chatDraft.message = String(event.currentTarget.value || "");
            state.chatDraft.dirty = true;
          };
          createRenderEffect((_p$) => {
            var _v$7 = tr(locale(), "给当前选中的 Agent 发一条消息", "Send a message to the selected agent"), _v$8 = !chatCapability().enabled;
            _v$7 !== _p$.e && setAttribute(_el$149, "placeholder", _p$.e = _v$7);
            _v$8 !== _p$.t && (_el$149.disabled = _p$.t = _v$8);
            return _p$;
          }, {
            e: void 0,
            t: void 0
          });
          createRenderEffect(() => _el$149.value = state.chatDraft.message);
          return _el$147;
        })(), (() => {
          var _el$150 = _tmpl$30(), _el$151 = _el$150.firstChild;
          _el$151.$$click = () => sendAgentChat(agentId(), state.chatDraft.message);
          insert(_el$151, () => tr(locale(), "发送聊天", "Send Chat"));
          createRenderEffect(() => _el$151.disabled = !chatCapability().enabled);
          return _el$150;
        })(), createComponent(Show, {
          get when() {
            return chatFeedback();
          },
          get fallback() {
            return createComponent(EmptyState, {
              get children() {
                return tr(locale(), "还没有聊天反馈。", "No chat feedback yet.");
              }
            });
          },
          children: (feedback) => createComponent(FeedbackCard, {
            get feedback() {
              return feedback();
            },
            get display() {
              return chatFeedbackDisplay();
            }
          })
        }), (() => {
          var _el$152 = _tmpl$31(), _el$153 = _el$152.firstChild, _el$154 = _el$153.nextSibling;
          insert(_el$153, () => tr(locale(), "消息流", "Message Flow"));
          insert(_el$154, createComponent(Show, {
            get when() {
              return chatHistory().length > 0;
            },
            get fallback() {
              return createComponent(EmptyState, {
                get children() {
                  return tr(locale(), "这个 Agent 还没有聊天历史。", "No chat history for this agent yet.");
                }
              });
            },
            get children() {
              return createComponent(For, {
                get each() {
                  return chatHistory();
                },
                children: (entry) => createComponent(EventCard, {
                  get title() {
                    return memo(() => entry.source === "player")() ? `${tr(locale(), "玩家", "player")} → ${entry.targetAgentId || entry.agentId || "agent"}` : `${entry.agentId || "agent"} ${tr(locale(), "已发言", "spoke")}`;
                  },
                  get badge() {
                    return `tick=${Number(entry.tick || 0)}`;
                  },
                  get meta() {
                    return `speaker=${entry.speaker || entry.playerId || "-"} · location=${entry.locationId || "-"}`;
                  },
                  get children() {
                    return createComponent(JsonBlock, {
                      value: entry
                    });
                  }
                })
              });
            }
          }));
          return _el$152;
        })()];
      }
    }), null);
    insert(_el$143, createComponent(PanelSection, {
      get title() {
        return tr(locale(), "高级 Prompt 设置", "Advanced Prompt Settings");
      },
      get eyebrow() {
        return tr(locale(), "高级控制", "Advanced Controls");
      },
      get meta() {
        return tr(locale(), "保留 operator 级 prompt 控制，但默认收起，不与玩家主路径竞争。", "Operator-level prompt controls stay available here, but collapsed by default so they do not compete with the player path.");
      },
      get children() {
        return [(() => {
          var _el$155 = _tmpl$14();
          insert(_el$155, createComponent(Badge, {
            get children() {
              return `activePrompt=v${promptVersionState().currentVersion}`;
            }
          }), null);
          insert(_el$155, createComponent(Badge, {
            get children() {
              return `nextRollback=v${promptVersionState().nextRollbackTargetVersion}`;
            }
          }), null);
          insert(_el$155, createComponent(Show, {
            get when() {
              return promptVersionState().restoredFromVersion != null;
            },
            get children() {
              return createComponent(Badge, {
                get children() {
                  return `restoredFrom=v${promptVersionState().restoredFromVersion}`;
                }
              });
            }
          }), null);
          insert(_el$155, createComponent(Badge, {
            get ["class"]() {
              return promptOverridesVisible() ? "badge badge--good" : "badge";
            },
            get children() {
              return memo(() => !!promptOverridesVisible())() ? tr(locale(), "状态=已展开", "state=expanded") : tr(locale(), "状态=默认收起", "state=hidden_by_default");
            }
          }), null);
          insert(_el$155, createComponent(Badge, {
            get children() {
              return tr(locale(), "本地设置持久化", "locally persisted");
            }
          }), null);
          return _el$155;
        })(), createComponent(EmptyState, {
          get children() {
            return promptSettingsSummary();
          }
        }), (() => {
          var _el$156 = _tmpl$32(), _el$157 = _el$156.firstChild;
          _el$157.$$click = () => togglePromptOverridesVisible();
          insert(_el$157, promptSettingsButtonLabel);
          return _el$156;
        })()];
      }
    }), null);
    insert(_el$143, createComponent(Show, {
      get when() {
        return promptOverridesVisible();
      },
      get children() {
        return createComponent(PanelSection, {
          title: "Prompt Overrides",
          get children() {
            return [(() => {
              var _el$158 = _tmpl$4();
              insert(_el$158, () => promptVersionState().summary);
              return _el$158;
            })(), (() => {
              var _el$159 = _tmpl$4();
              insert(_el$159, () => promptVersionState().detail);
              return _el$159;
            })(), createComponent(Show, {
              get when() {
                return memo(() => !!authSurface().capabilities.prompt_control.enabled)() && String(state.hostedAccess?.deployment_mode || "").trim() === "hosted_public_join";
              },
              get children() {
                var _el$160 = _tmpl$33(), _el$161 = _el$160.firstChild, _el$162 = _el$161.nextSibling;
                insert(_el$161, () => tr(locale(), "后端审批码", "Backend Approval Code"));
                _el$162.$$input = (event) => {
                  state.strongAuth.approvalCode = String(event.currentTarget.value || "");
                };
                createRenderEffect(() => _el$162.value = state.strongAuth.approvalCode || "");
                return _el$160;
              }
            }), (() => {
              var _el$163 = _tmpl$34(), _el$164 = _el$163.firstChild, _el$165 = _el$164.nextSibling;
              insert(_el$164, () => tr(locale(), "System Prompt 覆盖", "System Prompt Override"));
              _el$165.$$input = (event) => {
                state.promptDraft.systemPrompt = String(event.currentTarget.value || "");
                state.promptDraft.dirty = true;
              };
              createRenderEffect(() => _el$165.disabled = !promptCapability().enabled);
              createRenderEffect(() => _el$165.value = state.promptDraft.systemPrompt);
              return _el$163;
            })(), (() => {
              var _el$166 = _tmpl$35(), _el$167 = _el$166.firstChild, _el$168 = _el$167.nextSibling;
              insert(_el$167, () => tr(locale(), "短期目标覆盖", "Short-Term Goal Override"));
              _el$168.$$input = (event) => {
                state.promptDraft.shortTermGoal = String(event.currentTarget.value || "");
                state.promptDraft.dirty = true;
              };
              createRenderEffect(() => _el$168.disabled = !promptCapability().enabled);
              createRenderEffect(() => _el$168.value = state.promptDraft.shortTermGoal);
              return _el$166;
            })(), (() => {
              var _el$169 = _tmpl$36(), _el$170 = _el$169.firstChild, _el$171 = _el$170.nextSibling;
              insert(_el$170, () => tr(locale(), "长期目标覆盖", "Long-Term Goal Override"));
              _el$171.$$input = (event) => {
                state.promptDraft.longTermGoal = String(event.currentTarget.value || "");
                state.promptDraft.dirty = true;
              };
              createRenderEffect(() => _el$171.disabled = !promptCapability().enabled);
              createRenderEffect(() => _el$171.value = state.promptDraft.longTermGoal);
              return _el$169;
            })(), (() => {
              var _el$172 = _tmpl$37(), _el$173 = _el$172.firstChild, _el$174 = _el$173.nextSibling;
              _el$173.$$click = () => sendPromptControl("preview", null);
              insert(_el$173, () => tr(locale(), "预览 Prompt", "Preview Prompt"));
              _el$174.$$click = () => sendPromptControl("apply", null);
              insert(_el$174, () => tr(locale(), "应用 Prompt", "Apply Prompt"));
              createRenderEffect((_p$) => {
                var _v$9 = !promptCapability().enabled, _v$0 = !promptCapability().enabled;
                _v$9 !== _p$.e && (_el$173.disabled = _p$.e = _v$9);
                _v$0 !== _p$.t && (_el$174.disabled = _p$.t = _v$0);
                return _p$;
              }, {
                e: void 0,
                t: void 0
              });
              return _el$172;
            })(), (() => {
              var _el$175 = _tmpl$38(), _el$176 = _el$175.firstChild, _el$177 = _el$176.firstChild, _el$178 = _el$177.nextSibling, _el$179 = _el$176.nextSibling;
              insert(_el$177, () => tr(locale(), "下一次回滚目标版本", "Next Rollback Target Version"));
              _el$178.$$input = (event) => {
                const nextValue = Number(event.currentTarget.value || 0);
                state.promptDraft.rollbackTargetVersion = Math.max(0, Math.floor(nextValue || 0));
                requestRender();
              };
              _el$179.$$click = () => {
                sendPromptControl("rollback", {
                  toVersion: Number(state.promptDraft.rollbackTargetVersion || 0)
                });
              };
              insert(_el$179, () => tr(locale(), "回滚 Prompt", "Rollback Prompt"));
              createRenderEffect((_p$) => {
                var _v$1 = !promptCapability().enabled, _v$10 = !promptCapability().enabled;
                _v$1 !== _p$.e && (_el$178.disabled = _p$.e = _v$1);
                _v$10 !== _p$.t && (_el$179.disabled = _p$.t = _v$10);
                return _p$;
              }, {
                e: void 0,
                t: void 0
              });
              createRenderEffect(() => _el$178.value = Number(state.promptDraft.rollbackTargetVersion || 0));
              return _el$175;
            })(), createComponent(Show, {
              get when() {
                return promptFeedback();
              },
              get fallback() {
                return createComponent(EmptyState, {
                  get children() {
                    return tr(locale(), "还没有 Prompt 反馈。", "No prompt feedback yet.");
                  }
                });
              },
              children: (feedback) => createComponent(FeedbackCard, {
                get feedback() {
                  return feedback();
                },
                get display() {
                  return promptFeedbackDisplay();
                }
              })
            }), createComponent(Show, {
              get when() {
                return state.strongAuth.lastGrantActionId;
              },
              get children() {
                return createComponent(EmptyState, {
                  get children() {
                    return `lastGrant=${state.strongAuth.lastGrantActionId} expiresAt=${state.strongAuth.lastGrantExpiresAtUnixMs || "-"}`;
                  }
                });
              }
            }), createComponent(Show, {
              get when() {
                return state.strongAuth.lastGrantError;
              },
              get children() {
                return createComponent(EmptyState, {
                  style: "color:var(--bad);",
                  get children() {
                    return state.strongAuth.lastGrantError;
                  }
                });
              }
            })];
          }
        });
      }
    }), null);
    insert(_el$143, createComponent(PanelSection, {
      get title() {
        return tr(locale(), "资产 / 治理 Lane", "Asset / Governance Lane");
      },
      get eyebrow() {
        return tr(locale(), "后置能力", "Deferred Surface");
      },
      get meta() {
        return tr(locale(), "这类能力保留在右侧底部，只作为边界说明，不再抢占聊天与主玩法路径。", "These capabilities stay at the bottom of the right column as boundary guidance instead of competing with chat and the main player path.");
      },
      get children() {
        return [(() => {
          var _el$180 = _tmpl$14();
          insert(_el$180, createComponent(Badge, {
            get ["class"]() {
              return mainTokenTransferCapability().enabled ? "badge badge--good" : "badge badge--warn";
            },
            get children() {
              return `main_token_transfer=${assetLaneStatusText()}`;
            }
          }), null);
          insert(_el$180, createComponent(Badge, {
            get children() {
              return `required_auth=${mainTokenTransferPolicy()?.required_auth || "-"}`;
            }
          }), null);
          insert(_el$180, createComponent(Badge, {
            get children() {
              return `availability=${mainTokenTransferPolicy()?.availability || "-"}`;
            }
          }), null);
          return _el$180;
        })(), createComponent(EmptyState, {
          get children() {
            return assetLaneDetail();
          }
        }), createComponent(EmptyState, {
          get children() {
            return mainTokenTransferPolicy()?.reason || tr(locale(), "当前 lane 没有 main_token_transfer 的 hosted action policy。", "No hosted action policy is available for main_token_transfer on this lane.");
          }
        }), (() => {
          var _el$181 = _tmpl$39(), _el$182 = _el$181.firstChild;
          insert(_el$182, () => tr(locale(), "主代币转账（这里暂未开放）", "Main Token Transfer (Not Exposed Here Yet)"));
          return _el$181;
        })()];
      }
    }), null);
    return _el$143;
  })();
}
function DetailsPanel() {
  const locale = () => uiLocale();
  const gameplaySummary = () => buildGameplaySummary(locale());
  const worldScaleSurface = () => buildWorldScaleSurface(locale());
  const selectedLabel = () => state.selectedKind && state.selectedId ? `${state.selectedKind}:${state.selectedId}` : tr(locale(), "未选择", "nothing selected");
  const snapshotSummary = () => ({
    config: state.snapshot?.config || null,
    counts: {
      agents: Object.keys(state.snapshot?.model?.agents || {}).length,
      locations: Object.keys(state.snapshot?.model?.locations || {}).length,
      promptProfiles: Object.keys(state.snapshot?.model?.agent_prompt_profiles || {}).length,
      executionDebugContexts: Object.keys(state.snapshot?.model?.agent_execution_debug_contexts || {}).length
    },
    metrics: state.metrics,
    hostedAccess: clone(state.hostedAccess)
  });
  const snapshotCounts = () => ({
    agents: Object.keys(state.snapshot?.model?.agents || {}).length,
    locations: Object.keys(state.snapshot?.model?.locations || {}).length,
    promptProfiles: Object.keys(state.snapshot?.model?.agent_prompt_profiles || {}).length,
    executionDebugContexts: Object.keys(state.snapshot?.model?.agent_execution_debug_contexts || {}).length
  });
  const hasSnapshotDiagnostics = () => !!state.snapshot || !!state.metrics || !!state.hostedAccess;
  return (() => {
    var _el$183 = _tmpl$42(), _el$184 = _el$183.firstChild, _el$185 = _el$184.nextSibling, _el$186 = _el$185.firstChild, _el$187 = _el$186.nextSibling, _el$188 = _el$187.nextSibling, _el$189 = _el$188.firstChild, _el$190 = _el$189.nextSibling, _el$191 = _el$190.nextSibling, _el$192 = _el$191.firstChild, _el$193 = _el$192.nextSibling;
    insert(_el$184, createComponent(Badge, {
      "class": "badge badge--accent",
      get children() {
        return tr(locale(), "当前命令目标", "Current Command Target");
      }
    }), null);
    insert(_el$184, createComponent(Badge, {
      get children() {
        return selectedLabel();
      }
    }), null);
    insert(_el$183, createComponent(InteractionPanel, {}), _el$185);
    insert(_el$183, createComponent(Show, {
      get when() {
        return state.selectedObject;
      },
      get fallback() {
        return memo(() => gameplaySummary()?.blockerKind === "runtime_snapshot_empty_entities")() ? createComponent(EmptyEntityRecoveryCard, {
          get locale() {
            return locale();
          },
          gameplay: gameplaySummary,
          get title() {
            return tr(locale(), "对象明细暂时不可用", "Object Details Are Temporarily Unavailable");
          }
        }) : createComponent(EmptyState, {
          get children() {
            return tr(locale(), "请先从左侧列表选一个 Agent 或地点。", "Select an agent or location from the left list.");
          }
        });
      },
      children: (selected) => createComponent(DiagnosticDetails, {
        get locale() {
          return locale();
        },
        get label() {
          return tr(locale(), "展开对象原始明细", "Expand Raw Object Details");
        },
        get note() {
          return tr(locale(), "默认只保留交互面；只有在核查快照字段或诊断对象结构时再展开原始 JSON。", "The interaction surface stays in front by default. Expand raw JSON only when you need to inspect snapshot fields or diagnose object shape.");
        },
        value: () => clone(selected())
      })
    }), _el$185);
    insert(_el$186, () => tr(locale(), "世界规模", "World Scale"));
    insert(_el$187, createComponent(Badge, {
      get children() {
        return `agents=${snapshotCounts().agents}`;
      }
    }), null);
    insert(_el$187, createComponent(Badge, {
      get children() {
        return `locations=${snapshotCounts().locations}`;
      }
    }), null);
    insert(_el$187, createComponent(Badge, {
      get children() {
        return `promptProfiles=${snapshotCounts().promptProfiles}`;
      }
    }), null);
    insert(_el$187, createComponent(Badge, {
      get children() {
        return `debugContexts=${snapshotCounts().executionDebugContexts}`;
      }
    }), null);
    insert(_el$188, createComponent(MetricCard, {
      get label() {
        return tr(locale(), "物理真值单位", "Canonical Physical Unit");
      },
      get value() {
        return worldScaleSurface().physicalTruth.canonicalUnitLabel || "-";
      },
      get children() {
        return createComponent(Badge, {
          get children() {
            return tr(locale(), "整数厘米", "integer centimeters");
          }
        });
      }
    }), _el$189);
    insert(_el$189, () => worldScaleSurface().physicalTruth.canonicalUnitDetail);
    insert(_el$188, createComponent(MetricCard, {
      get label() {
        return tr(locale(), "世界边界", "World Bounds");
      },
      get value() {
        return worldScaleSurface().physicalTruth.worldBoundsLabel || tr(locale(), "未发布", "not published");
      },
      get children() {
        return createComponent(Badge, {
          get children() {
            return tr(locale(), "snapshot.config.space", "snapshot.config.space");
          }
        });
      }
    }), _el$190);
    insert(_el$190, () => worldScaleSurface().physicalTruth.worldBoundsDetail);
    insert(_el$188, createComponent(Show, {
      get when() {
        return worldScaleSurface().physicalTruth.anchor;
      },
      children: (anchor) => createComponent(EventCard, {
        get title() {
          return anchor().label;
        },
        get badge() {
          return anchor().kind;
        },
        badgeClass: "badge badge--accent",
        get meta() {
          return `id=${anchor().id}${anchor().locationId ? ` · location=${anchor().locationId}` : ""}`;
        },
        get children() {
          return [(() => {
            var _el$200 = _tmpl$13();
            insert(_el$200, () => anchor().positionLabel || tr(locale(), "缺少可读坐标。", "Missing readable coordinates."));
            return _el$200;
          })(), createComponent(Show, {
            get when() {
              return anchor().radiusLabel;
            },
            get children() {
              var _el$201 = _tmpl$43(), _el$202 = _el$201.firstChild;
              insert(_el$201, () => tr(locale(), "地点半径真值", "Location radius truth"), _el$202);
              insert(_el$201, () => anchor().radiusLabel, null);
              return _el$201;
            }
          })];
        }
      })
    }), _el$191);
    insert(_el$192, () => tr(locale(), "最近距离样本", "Nearest Distance Samples"));
    insert(_el$193, createComponent(Show, {
      get when() {
        return worldScaleSurface().physicalTruth.nearestLocations.length > 0;
      },
      get fallback() {
        return createComponent(EmptyState, {
          get children() {
            return tr(locale(), "当前没有足够的地点数据来给出距离样本。", "The current snapshot does not expose enough locations to show distance samples.");
          }
        });
      },
      get children() {
        return createComponent(For, {
          get each() {
            return worldScaleSurface().physicalTruth.nearestLocations;
          },
          children: (location) => createComponent(EventCard, {
            get title() {
              return location.name;
            },
            get badge() {
              return location.distanceLabel || "-";
            },
            badgeClass: "badge badge--good",
            get meta() {
              return `id=${location.id}`;
            },
            get children() {
              return [(() => {
                var _el$203 = _tmpl$43(), _el$204 = _el$203.firstChild;
                insert(_el$203, () => tr(locale(), "真实距离", "Physical distance"), _el$204);
                insert(_el$203, () => location.distanceLabel || "-", null);
                return _el$203;
              })(), createComponent(Show, {
                get when() {
                  return location.radiusLabel;
                },
                get children() {
                  var _el$205 = _tmpl$43(), _el$206 = _el$205.firstChild;
                  insert(_el$205, () => tr(locale(), "地点半径", "Location radius"), _el$206);
                  insert(_el$205, () => location.radiusLabel, null);
                  return _el$205;
                }
              })];
            }
          })
        });
      }
    }));
    insert(_el$188, createComponent(EventCard, {
      get title() {
        return tr(locale(), "表现层说明", "Presentation Notes");
      },
      get badge() {
        return tr(locale(), "不要误读 marker", "Do not trust marker size");
      },
      badgeClass: "badge badge--warn",
      get children() {
        return [(() => {
          var _el$194 = _tmpl$13();
          insert(_el$194, () => worldScaleSurface().presentationScale.markerTruthNote);
          return _el$194;
        })(), (() => {
          var _el$195 = _tmpl$4();
          insert(_el$195, () => worldScaleSurface().presentationScale.zoomTruthNote);
          return _el$195;
        })(), (() => {
          var _el$196 = _tmpl$4();
          insert(_el$196, () => worldScaleSurface().presentationScale.softwareSafeNote);
          return _el$196;
        })()];
      }
    }), null);
    insert(_el$188, createComponent(EmptyState, {
      get children() {
        return tr(locale(), "主状态已经在中间的“世界摘要”里展示；这里现在专门保留“厘米真值 vs 表现层夸张”的读图锚点，原始快照仍按需展开。", "The main runtime state already lives in World Summary; this section now reserves the reading anchors for centimeter truth vs presentation exaggeration, while raw snapshots stay collapsible.");
      }
    }), null);
    insert(_el$185, createComponent(Show, {
      get when() {
        return hasSnapshotDiagnostics();
      },
      get children() {
        return createComponent(DiagnosticDetails, {
          get locale() {
            return locale();
          },
          get label() {
            return tr(locale(), "展开原始快照诊断", "Expand Raw Snapshot Diagnostics");
          },
          get note() {
            return tr(locale(), "只在需要排查快照结构或 hosted access 原始字段时展开。", "Expand only when you need to inspect the raw snapshot shape or hosted access fields.");
          },
          value: snapshotSummary
        });
      }
    }), null);
    insert(_el$183, createComponent(Show, {
      get when() {
        return state.lastError;
      },
      get children() {
        var _el$197 = _tmpl$41(), _el$198 = _el$197.firstChild, _el$199 = _el$198.nextSibling;
        insert(_el$198, () => tr(locale(), "最近错误", "Last Error"));
        insert(_el$199, () => state.lastError);
        return _el$197;
      }
    }), null);
    return _el$183;
  })();
}
function AppShell() {
  const locale = () => uiLocale();
  return [createComponent(MobileJumpRail, {}), (() => {
    var _el$207 = _tmpl$44(), _el$208 = _el$207.firstChild, _el$209 = _el$208.firstChild, _el$210 = _el$209.nextSibling, _el$211 = _el$210.nextSibling, _el$212 = _el$208.nextSibling;
    insert(_el$209, () => tr(locale(), "导航", "Navigate"));
    insert(_el$210, () => tr(locale(), "目标", "Targets"));
    insert(_el$211, () => tr(locale(), "先锁定对象，再进入世界舞台或右侧命令面。", "Lock onto a target first, then move into the stage or command surface."));
    insert(_el$212, createComponent(TargetsPanel, {}));
    return _el$207;
  })(), (() => {
    var _el$213 = _tmpl$45(), _el$214 = _el$213.firstChild, _el$215 = _el$214.firstChild;
    insert(_el$215, createComponent(WorldStageHero, {}), null);
    insert(_el$215, createComponent(PixelWorldHost, {
      get locale() {
        return locale();
      }
    }), null);
    insert(_el$215, createComponent(WorldSummaryPanel, {}), null);
    return _el$213;
  })(), (() => {
    var _el$216 = _tmpl$46(), _el$217 = _el$216.firstChild, _el$218 = _el$217.firstChild, _el$219 = _el$218.nextSibling, _el$220 = _el$219.nextSibling, _el$221 = _el$217.nextSibling;
    insert(_el$218, () => tr(locale(), "指挥与核查", "Command and Inspect"));
    insert(_el$219, () => tr(locale(), "交互与明细", "Interact and Inspect"));
    insert(_el$220, () => tr(locale(), "只有锁定目标后才进入这里。聊天优先，Prompt 与对象核查继续后置。", "Enter this column only after locking a target. Chat comes first; prompt controls and raw inspection stay behind it."));
    insert(_el$221, createComponent(DetailsPanel, {}));
    return _el$216;
  })()];
}
const app = document.getElementById("app");
if (!app) {
  throw new Error("viewer root #app is missing");
}
let dispose = render$1(() => createComponent(AppShell, {}), app);
setRenderHook(() => {
  dispose();
  app.textContent = "";
  dispose = render$1(() => createComponent(AppShell, {}), app);
});
initializeSoftwareSafeCore();
delegateEvents(["click", "input"]);
