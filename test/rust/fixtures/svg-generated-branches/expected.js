import { textRenderFn as textRenderFn$jg$, renderIntlText as renderIntlText$jg$, renderIntlTextWithParams as renderIntlTextWithParams$jg$, renderIntlRichText as renderIntlRichText$jg$, createEle as createEle$jg$, createEleA as createEleA$jg$, createSVGEle as createSVGEle$jg$, createSVGEleA as createSVGEleA$jg$, CONTEXT as CONTEXT$jg$, createTextNode as createTextNode$jg$, setTextContent as setTextContent$jg$, vm as vm$jg$, addEvent as addEvent$jg$, setAttribute as setAttribute$jg$, setRefForComponent as setRefForComponent$jg$, ROOT_NODES as ROOT_NODES$jg$, ComponentHost as ComponentHost$jg$, watchForDOMEventBind as watchForDOMEventBind$jg$, watchPathForDOMEventBind as watchPathForDOMEventBind$jg$, NON_ROOT_COMPONENT_NODES as NON_ROOT_COMPONENT_NODES$jg$, renderSlotFunction as renderSlotFunction$jg$, renderFunctionComponent as renderFunctionComponent$jg$, IntlWatcher as IntlWatcher$jg$, PathWatcher as PathWatcher$jg$, DymPathWatcher as DymPathWatcher$jg$, ExprWatcher as ExprWatcher$jg$, watchForRender as watchForRender$jg$, watchPathForRender as watchPathForRender$jg$, watchPathForRender2 as watchPathForRender2$jg$, If as If$jg$, For as For$jg$, KEY_DATA as KEY_DATA$jg$, KEY_INDEX as KEY_INDEX$jg$ } from "jinge";
import { vm } from 'jinge';
const state = vm({
    on: true,
    items: [
        {
            id: 1,
            x: 10
        }
    ]
});
export function A(attrs$jg$, rthost$jg$) {
    return [
        (()=>{
            const $jg$ = createSVGEle$jg$("svg", ...(()=>{
                const attrs$jg$ = vm$jg$({
                    "slot:default": (attrs$jg$, host$jg$)=>[
                            (()=>{
                                const $jg$ = createSVGEleA$jg$("path", {
                                    "d": "M0 0L10 10"
                                });
                                host$jg$[ROOT_NODES$jg$].push($jg$);
                                return $jg$;
                            })()
                        ]
                });
                watchPathForRender$jg$(state, [
                    "on"
                ], (v)=>attrs$jg$["expect"] = v, rthost$jg$);
                const $jg$ = new ComponentHost$jg$(rthost$jg$[CONTEXT$jg$]);
                rthost$jg$[NON_ROOT_COMPONENT_NODES$jg$].push($jg$);
                return renderFunctionComponent$jg$($jg$, If$jg$, attrs$jg$);
            })(), ...(()=>{
                const attrs$jg$ = vm$jg$({
                    "key": "id",
                    "slot:default": (each$jg$0, host$jg$)=>[
                            (()=>{
                                const $jg$ = createSVGEleA$jg$("circle", {
                                    "r": "5"
                                });
                                watchPathForRender$jg$(each$jg$0, [
                                    "data",
                                    "x"
                                ], (v)=>setAttribute$jg$($jg$, "cx", v), host$jg$);
                                host$jg$[ROOT_NODES$jg$].push($jg$);
                                return $jg$;
                            })()
                        ]
                });
                watchPathForRender$jg$(state, [
                    "items"
                ], (v)=>attrs$jg$["loop"] = v, rthost$jg$);
                const $jg$ = new ComponentHost$jg$(rthost$jg$[CONTEXT$jg$]);
                rthost$jg$[NON_ROOT_COMPONENT_NODES$jg$].push($jg$);
                return renderFunctionComponent$jg$($jg$, For$jg$, attrs$jg$);
            })());
            rthost$jg$[ROOT_NODES$jg$].push($jg$);
            return $jg$;
        })()
    ];
}
