import { textRenderFn as textRenderFn$jg$, renderIntlText as renderIntlText$jg$, renderIntlTextWithParams as renderIntlTextWithParams$jg$, renderIntlRichText as renderIntlRichText$jg$, createEle as createEle$jg$, createEleA as createEleA$jg$, createSVGEle as createSVGEle$jg$, createSVGEleA as createSVGEleA$jg$, CONTEXT as CONTEXT$jg$, createTextNode as createTextNode$jg$, setTextContent as setTextContent$jg$, vm as vm$jg$, addEvent as addEvent$jg$, setAttribute as setAttribute$jg$, setRefForComponent as setRefForComponent$jg$, ROOT_NODES as ROOT_NODES$jg$, ComponentHost as ComponentHost$jg$, watchForDOMEventBind as watchForDOMEventBind$jg$, watchPathForDOMEventBind as watchPathForDOMEventBind$jg$, NON_ROOT_COMPONENT_NODES as NON_ROOT_COMPONENT_NODES$jg$, renderSlotFunction as renderSlotFunction$jg$, renderFunctionComponent as renderFunctionComponent$jg$, IntlWatcher as IntlWatcher$jg$, PathWatcher as PathWatcher$jg$, DymPathWatcher as DymPathWatcher$jg$, ExprWatcher as ExprWatcher$jg$, watchForRender as watchForRender$jg$, watchPathForRender as watchPathForRender$jg$, watchPathForRender2 as watchPathForRender2$jg$, If as If$jg$, For as For$jg$, KEY_DATA as KEY_DATA$jg$, KEY_INDEX as KEY_INDEX$jg$ } from "jinge";
import { vm } from 'jinge';
const key = 'name';
const state = vm({
    name: 'Alice'
});
export function A(attrs$jg$, rthost$jg$) {
    return [
        (()=>{
            const $jg$ = createEle$jg$("p", (()=>{
                const $jg$ = createTextNode$jg$("");
                watchPathForRender$jg$(state, [
                    key
                ], ($jg$v)=>setTextContent$jg$($jg$, $jg$v), rthost$jg$);
                return $jg$;
            })());
            rthost$jg$[ROOT_NODES$jg$].push($jg$);
            return $jg$;
        })()
    ];
}
