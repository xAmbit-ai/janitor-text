use std::io::Cursor;

use html5ever::{parse_document, tendril::TendrilSink, tree_builder::TreeSink};
use markup5ever_rcdom::{Handle, NodeData, RcDom};
use pulldown_cmark::{Parser, Event, Tag};
use whatlang::{detect, Script, Lang};

pub fn clean_text_with_markdown(text: &str) -> (Vec<String>, usize, usize) {
    let parsed = Parser::new(text);
    let mut texts = vec![];

    let mut added = 0;
    let mut ignored = 0;

    let mut ignore = false;
    for evt in parsed {
        match evt {
            Event::Start(e) => {
                match e {
                    Tag::Heading(_, _, _) => { ignore = true },
                    _ => {
                        // println!("Start: {e:?}")
                    }
                }
            }
            Event::End(_) => {
                if ignore {
                    ignore = false;
                    continue;
                }
                // println!("End Tag: {e:?}");
            },
            Event::Text(t) => {
                if ignore {
                    continue;
                }

                let (text, a, i) = clean_text(&t);
                added += a;
                ignored += i;

                for t in text {
                    if !texts.is_empty() && texts.last().unwrap() == &t {
                        continue;
                    }

                    push_cleaned_text(&mut texts, t);
                }
                push_cleaned_text(&mut texts, "[SEP]".to_owned());
            }
            Event::Code(_) => {
                if ignore {
                    continue;
                }

                _ = push_cleaned_text(&mut texts, "code".to_owned())
            },
            Event::HardBreak|Event::SoftBreak => {
                if ignore {
                    continue;
                }

                _ = push_cleaned_text(&mut texts, "[SEP]".to_owned())
            },
            Event::Html(h) => {
                if ignore {
                    continue;
                }

                let (text, a, i) = clean_text_with_html(&h);
                added += a;
                ignored += i;

                for t in text {
                    if !texts.is_empty() && texts.last().unwrap() == &t {
                        continue;
                    }

                    push_cleaned_text(&mut texts, t);
                }
                push_cleaned_text(&mut texts, "[SEP]".to_owned());
            },
            Event::FootnoteReference(f) => {
                if ignore {
                    continue;
                }
                
                println!("Footnote: {f:?}");
            },
            Event::Rule => {
                println!("Rule");
            },
            Event::TaskListMarker(t) => {
                if ignore {
                    continue;
                }
                println!("Tasklist: {t:?}");
            }
        }
    }

    (texts, added, ignored)
}

pub fn clean_text_with_html(text: &str) -> (Vec<String>,usize, usize) {
    let rgx = regex::Regex::new(r"(\{(.|\n|\r\r)*\})|(<code>(.|\n|\r\r)*</code>)|(\?php(.|\n)?)")
        .unwrap();
    let text = text.to_owned();
    let txt = rgx
        .replace_all(&parse_html(&text), "Section contained code.")
        .to_string();

    clean_text(&txt)
}

fn parse_html(s: &str) -> String {
    let mut s = Cursor::new(s.as_bytes());
    let mut dom = parse_document(RcDom::default(), Default::default())
        .from_utf8()
        .read_from(&mut s)
        .unwrap();

    let node = dom.get_document();
    let mut texts = vec![];
    parse_node(&node, &mut texts);

    texts.join(" . ")
}

fn parse_node(node: &Handle, texts: &mut Vec<String>) -> String {
    match node.data {
        NodeData::Text { ref contents } => {
            // println!("#text: {}", )
            let txt = contents.borrow().trim().escape_default().to_string();
            if !txt.is_empty() {
                texts.push(txt);
            }
        }

        // NodeData::Comment { ref contents } => {
        //     let txt = contents.trim().escape_default().to_string();
        //     if !txt.is_empty() {
        //         texts.push(txt);
        //     }
        // }

        NodeData::Element {
            ref name,
            // ref attrs,
            ..
        } => {
            if name.local.as_bytes() == "pre".as_bytes()
                || name.local.as_bytes() == "code".as_bytes()
            {
                return "code".to_string();
            }
        }

        NodeData::ProcessingInstruction { .. } => unreachable!(),
        _ => {}
    }

    for child in node.children.borrow().iter() {
        // walk(indent + 4, child);
        parse_node(child, texts);
    }

    "".to_string()
}

pub fn clean_text(input: &str) -> (Vec<String>, usize, usize) {
    let mut text = Vec::new();
    let mut last = String::new();
    let mut lastsplchar = ' ';

    let mut escaped = false;
    let mut added = 0;
    let mut ignored = 0;

    for c in input.chars() {
        // handelling consecutive punctuations
        if c.is_ascii_punctuation() && c == lastsplchar {
            continue;
        }
        lastsplchar = c;

        if c.is_whitespace() {
            if !last.is_empty() {
                if let Some(p) = push_cleaned_text(&mut text, last.trim().to_owned()) {
                    if !p {
                        ignored += 1;
                    } else {
                        added += 1;
                    }
                }

                last = String::new();
                if c != ' ' {
                    // text.push("[SEP]".to_string());
                    push_cleaned_text(&mut text, "[SEP]".to_owned());
                }
            }

            continue;
        }

        if c == '\\' {
            escaped = true;
            continue;
        }

        if escaped {
            escaped = false;
            if c == 'n' || c == 't' || c == 'r' {
                if !last.is_empty() {
                    if let Some(p) = push_cleaned_text(&mut text, last.trim().to_owned()) {
                        if !p {
                            ignored += 1;
                        } else {
                            added += 1;
                        }
                    }
                    last = String::new();
                }
                continue;
            }
        }

        if c == '!'
            || c == '?'
            || c == ','
            || c == ';'
            || c == '('
            || c == ')'
            || c == '<'
            || c == '>'
            || c == '$'
            || c == '&'
            || c == '\''
            || c == '"'
            || c == '['
            || c == ']'
        {
            if !last.is_empty() {
                if let Some(p) = push_cleaned_text(&mut text, last.trim().to_owned()) {
                    if !p {
                        ignored += 1;
                    } else {
                        added += 1;
                    }
                }
                
                last = String::new();
            }

            push_cleaned_text(&mut text, c.to_string());
            continue;
        }

        if (c == ':' || c == '.') && !last.starts_with("http") {
            if !last.is_empty() {

                if let Some(p) = push_cleaned_text(&mut text, last.trim().to_owned()) {
                    if !p {
                        ignored += 1;
                    } else {
                        added += 1;
                    }
                }
                last = String::new();
            }

            push_cleaned_text(&mut text, c.to_string());
            continue;
        }

        last.push(c);
    }

    if !last.is_empty() {
        if let Some(p) = push_cleaned_text(&mut text, last.trim().to_owned()) {
            if !p {
                ignored += 1;
            } else {
                added += 1;
            }
        }
    }

    (text, added, ignored)
}

fn push_cleaned_text(d: &mut Vec<String>, txt: String) -> Option<bool> {
    let mut txt = txt.trim();
    if txt.starts_with("http:/") || txt.starts_with("https:/") {
        d.push("link".to_string());
        return Some(true);
    }

    if let Some(p) = d.last() {
        // ignoring if last two words are exactly the same
        if p == &txt || (is_special_punctuation(p) && (txt == "[SEP]" || txt == "[CLS]")) {
            return Some(true);
        }
    }

    let charcount = txt.chars().count();

    if charcount > 1 {
        if let Some(lang) = detect(&txt) {
            if lang.script() != Script::Latin || (lang.lang() != Lang::Eng && lang.confidence() > 0.6) {
                return Some(false);
            }
        }
    }
    
    if txt.chars().count() > 32 {
        println!("Replacing with random: {txt}");
        txt = "random-word";
    }

    d.push(txt.to_owned());
    
    if charcount > 1 {
        Some(true)
    } else {
        None
    }
}

pub fn is_special_punctuation(txt: &str) -> bool {
    if txt.chars().count() != 1 {
        return false;
    }

    let c = txt.chars().next().unwrap();

    c == '.'
        || c == '!'
        || c == '?'
        || c == ','
        || c == ';'
        || c == '('
        || c == ')'
        || c == '<'
        || c == '>'
        || c == '$'
        || c == '&'
        || c == '\''
        || c == '"'
        || c == ':'
        || c == '['
        || c == ']'
        || c.is_whitespace()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_text() {
        let txt = r#"Highchart: in bar chart, how to increment a bar according to data?","<p>Code in concern: <a href=""http://jsfiddle.net/h6qrbpwo/10/"">http://jsfiddle.net/h6qrbpwo/10/</a></p> \

        <pre><code>$(function() {
          var chart;
          var d = 1;
          var index = 0;
        
          function getYValue(chartObj, seriesIndex, xValue) {
            var yValue = null;
            var points = chartObj.series[seriesIndex].points;
            for (var i = 0; i &lt; points.length; i++) {
                if(i == points.length - 1 &amp;&amp; points[i].x != xValue){    
                return 0;
              }
              yValue = points[i].y;
            }
            return yValue;
          }
          $('#b').click(function() {
            console.log(index);
            var d = getYValue(chart, index, 20.5);
            console.log(d);
            d++;
            console.log(d);
            chart.addSeries({
              grouping: false,
              data: [
                [20.5, d]
              ]
            });
            index ++;
          })
          chart = new Highcharts.Chart({
            chart: {
              type: 'column',
              renderTo: 'container'
            },
            title: {
              text: ''
            },
            xAxis: {
              min: 0,
              max: 100
            },
            credits: {
              enabled: false
            },
            series: [{
              name: '',
              data: [5, 3, 4, 7, 2]
            }]
          });
        });
        </code></pre>
        
        <p>(Note: this JSFiddle is just for demonstration purpose.)</p>
        
        <p>I would like to have a bar chart with bars with animated incrementation (i.e. only the part increased) instead of redrawing the whole bar.</p>
        <p>More details can be found at http://hello.world.com</p>
        <p>Thanks in advance.</p>
        "#;

        let res = clean_text_with_html(txt);

        println!("{res:?}");
        // assert_eq!(res, )
    }
    
    #[test]
    fn test_markdown() {
        let texts = [
            r#"<!--\r\nplease answer these questions before submitting your issue. thanks!\r\nfor questions please use one of our forums: https://cuelang.slack.com/\r\n-->\r\n\r\n### what version of cue are you using (`cue version`)?\r\n\r\n<pre>\r\n$ cue version\r\ncue version 0.3.0-beta.2 darwin/amd64\r\n</pre>\r\n\r\n<!--\r\nif you built from source, specify what git tag or commit was used.\r\n-->\r\n\r\n### does this issue reproduce with the latest release?\r\n\r\nyep.\r\n\r\n### what did you do?\r\n\r\n`cue eval -io uplot.cue` - original file is in [this gist](https://gist.github.com/sdboyer/5053e745b7c16579f7d59ccc8c340af3#file-uplot-cue).\r\n\r\n<!--\r\nif possible, provide a recipe for reproducing the error.\r\n-->\r\n\r\n\r\n\r\n### what did you expect to see?\r\n\r\nwell, not a panic 😉 the appropriate eval'd output.\r\n\r\n\r\n### what did you see instead?\r\n\r\n```\r\npanic: runtime error: invalid memory address or nil pointer dereference [recovered]\r\n\tpanic: runtime error: invalid memory address or nil pointer dereference\r\n[signal sigsegv: segmentation violation code=0x1 addr=0x38 pc=0x1254446]\r\n\r\ngoroutine 1 [running]:\r\ncuelang.org/go/cmd/cue/cmd.recovererror(0xc000213ec0)\r\n\t/users/mpvl/documents/dev/release/cue/cmd/cue/cmd/root.go:221 +0x95\r\npanic(0x17d2300, 0x1daee00)\r\n\t/usr/local/go/src/runtime/panic.go:969 +0x1b9\r\ncuelang.org/go/cue/ast.comments(0x0, 0x0, 0x0, 0x0, 0x1)\r\n\t/users/mpvl/documents/dev/release/cue/cue/ast/comments.go:19 +0x26\r\ncuelang.org/go/cue/ast.(*inspector).before(0xc000526f00, 0x0, 0x0, 0x1f86108, 0xc000526f00)\r\n\t/users/mpvl/documents/dev/release/cue/cue/ast/walk.go:228 +0xa7\r\ncuelang.org/go/cue/ast.walk(0x198ed80, 0xc000526f00, 0x0, 0x0)\r\n\t/users/mpvl/documents/dev/release/cue/cue/ast/walk.go:61 +0x63\r\ncuelang.org/go/cue/ast.walk(...)\r\n\t/users/mpvl/documents/dev/release/cue/cue/ast/walk.go:29\r\ncuelang.org/go/internal/core/export.striprefs(0x0, 0x0, 0xc0000fe900, 0x0)\r\n\t/users/mpvl/documents/dev/release/cue/internal/core/export/value.go:95 +0xa5\r\ncuelang.org/go/internal/core/export.(*exporter).structcomposite(0xc0000e8fc0, 0xc0000d5e60, 0x0, 0x0)\r\n\t/users/mpvl/documents/dev/release/cue/internal/core/export/value.go:413 +0x6a5\r\ncuelang.org/go/internal/core/export.(*exporter).vertex(0xc0000e8fc0, 0xc0000d5e60, 0xc0000d5e60, 0xc000296088)\r\n\t/users/mpvl/documents/dev/release/cue/internal/core/export/value.go:48 +0x55f\r\ncuelang.org/go/internal/core/export.(*exporter).structcomposite(0xc0000e8fc0, 0xc0000d55f0, 0x0, 0x0)\r\n\t/users/mpvl/documents/dev/release/cue/internal/core/export/value.go:419 +0x953\r\ncuelang.org/go/internal/core/export.(*exporter).vertex(0xc0000e8fc0, 0xc0000d55f0, 0x19a14c0, 0xc0000d55f0)\r\n\t/users/mpvl/documents/dev/release/cue/internal/core/export/value.go:48 +0x55f\r\ncuelang.org/go/internal/core/export.(*exporter).value(0xc0000e8fc0, 0x19a14c0, 0xc0000d55f0, 0x0, 0x0, 0x0, 0xc000399f28, 0x6)\r\n\t/users/mpvl/documents/dev/release/cue/internal/core/export/value.go:153 +0xab7\r\ncuelang.org/go/internal/core/export.(*profile).value(0xc000399f20, 0x199cb80, 0xc0000d2c20, 0xc000399f28, 0x6, 0x19a14c0, 0xc0000d55f0, 0x8, 0x189957a, 0x4, ...)\r\n\t/users/mpvl/documents/dev/release/cue/internal/core/export/export.go:204 +0x1c5\r\ncuelang.org/go/cue.value.syntax(0xc0004393d0, 0xc0000d55f0, 0xc000213c18, 0x4, 0x4, 0xc0003a20c0, 0x0)\r\n\t/users/mpvl/documents/dev/release/cue/cue/types.go:972 +0x205\r\ncuelang.org/go/cmd/cue/cmd.runeval(0xc00036b860, 0xc00036bfe0, 0x1, 0x2, 0x0, 0x0)\r\n\t/users/mpvl/documents/dev/release/cue/cmd/cue/cmd/eval.go:154 +0x687\r\ncuelang.org/go/cmd/cue/cmd.mkrune.func1(0xc0002b18c0, 0xc00036bfe0, 0x1, 0x2, 0x0, 0x0)\r\n\t/users/mpvl/documents/dev/release/cue/cmd/cue/cmd/root.go:46 +0x6c\r\ngithub.com/spf13/cobra.(*command).execute(0xc0002b18c0, 0xc00036bfc0, 0x2, 0x2, 0xc0002b18c0, 0xc00036bfc0)\r\n\t/users/mpvl/go/pkg/mod/github.com/spf13/cobra@v1.0.0/command.go:842 +0x47c\r\ngithub.com/spf13/cobra.(*command).executec(0xc0002b1080, 0x0, 0x0, 0x0)\r\n\t/users/mpvl/go/pkg/mod/github.com/spf13/cobra@v1.0.0/command.go:950 +0x375\r\ngithub.com/spf13/cobra.(*command).execute(...)\r\n\t/users/mpvl/go/pkg/mod/github.com/spf13/cobra@v1.0.0/command.go:887\r\ncuelang.org/go/cmd/cue/cmd.(*command).run(0xc00036b860, 0x199a280, 0xc0000a6008, 0x0, 0x0)\r\n\t/users/mpvl/documents/dev/release/cue/cmd/cue/cmd/root.go:206 +0x65\r\ncuelang.org/go/cmd/cue/cmd.mainerr(0x199a280, 0xc0000a6008, 0xc0000a4050, 0x3, 0x3, 0x18dc6c0, 0xc0002fdf48)\r\n\t/users/mpvl/documents/dev/release/cue/cmd/cue/cmd/root.go:145 +0x8a\r\ncuelang.org/go/cmd/cue/cmd.main(0xc00008c058)\r\n\t/users/mpvl/documents/dev/release/cue/cmd/cue/cmd/root.go:127 +0x9c\r\nmain.main()\r\n\t/users/mpvl/documents/dev/release/cue/cmd/cue/main.go:24 +0x25\r\n```\r\n"#,
            "### description\r\n\r\n<!-- provide a clear description of the issue including any relevant logs or screenshots. add `bug`, `enhancement` or other labels as appropriate. -->\r\nif an experiment configuration is sufficiently unusual that it cannot use `climatemachine.invoke!` (e.g. bickley jet), it should still be able to use the cli parsing functionality in the driver top-level.",
            "in gitlab by @infinitewarp on mar 10, 2020, 16:04\n\nwhat is the best solution for accessing data in a customer's azure account/tenant? how can we execute api requests in their azure account/tenant?\n\nthis is a followup to https://gitlab.com/cloudigrade/cloudigrade/-/issues/573 and https://gitlab.com/cloudigrade/cloudigrade/issues/514 and https://gitlab.com/cloudigrade/cloudigrade/-/issues/480\n\nas i recently described our aws iam workflow to our microsoft reps, their reply was:\n\n> [alfred] if you're in the same tenant i can see this being possible through our iam functionality. i know it's possible to create users in a tenant from outside the tenant, but i'm not sure about the level of permissions granted/how easy it is to grant the guest user sufficient permissions. also not sure about programmatic access policies for guest users ☹\n\nwe should review our notes from the previous investigation(s), try to find if there is anything better or newer we should consider, and either confirm the previous solution or propose a new solution.\n\noutput: another jupyter notebook or standalone python script to demonstrate how we can interact with a different azure account/tenant.\n\ntime box: 3 days.",
            "**which jobs are flaking**: `ci-kubernetes-kind-e2e-parallel`\r\n\r\n**which test(s) are flaking**: `persistentvolumes-local [volume type: dir-bindmounted] two pods mounting a local volume one after the other should be able to write from pod1 and read from pod2`\r\n\r\n**testgrid link**:  https://testgrid.k8s.io/sig-release-master-blocking#kind-master-parallel&include-filter-by-regex=bindmounted%5d%20two%20pods%20mounting%20a%20local%20volume%20one%20after%20the%20other%20should%20be%20able%20to%20write%20from%20pod1%20and%20read%20from%20pod2\r\n\r\n\r\n**reason for failure**:\r\n```\r\nstep: deleting pod pod-cfdd703e-6f3e-4303-9128-71470a513fc4 in namespace persistent-local-volumes-test-6369\r\nstep: creating pod2\r\nstep: creating a pod\r\njan 18 17:47:12.808: fail: unexpected error:\r\n    <*errors.errorstring | 0xc001da2b80>: {\r\n        s: \"pod \\\"pod-151f7372-edc8-4eb3-8f6c-d19124545742\\\" is not running: timed out waiting for the condition\",\r\n    }\r\n    pod \"pod-151f7372-edc8-4eb3-8f6c-d19124545742\" is not running: timed out waiting for the condition\r\noccurred\r\n```\r\n```\r\n  [volume type: dir-bindmounted]\r\n  test/e2e/storage/persistent_volumes-local.go:191\r\n    two pods mounting a local volume one after the other\r\n    test/e2e/storage/persistent_volumes-local.go:253\r\n      should be able to write from pod1 and read from pod2 [it]\r\n      test/e2e/storage/persistent_volumes-local.go:254\r\n      jan 18 17:47:12.808: unexpected error:\r\n      \r\n          <*errors.errorstring | 0xc001da2b80>: {\r\n              s: \"pod \\\"pod-151f7372-edc8-4eb3-8f6c-d19124545742\\\" is not running: timed out waiting for the condition\",\r\n          }\r\n          pod \"pod-151f7372-edc8-4eb3-8f6c-d19124545742\" is not running: timed out waiting for the condition\r\n      occurred\r\n      test/e2e/storage/persistent_volumes-local.go:784\r\n```\r\n**anything else we need to know**:\r\n\r\ntriage links: \r\nhttps://storage.googleapis.com/k8s-gubernator/triage/index.html?test=two%20pods%20mounting%20a%20local%20volume%20one%20after%20the%20other%20should%20be%20able%20to%20write%20from%20pod1%20and%20read%20from%20pod2\r\n\r\nspyglass links:\r\nhttps://prow.k8s.io/view/gcs/kubernetes-jenkins/logs/ci-kubernetes-kind-e2e-parallel/1351218547929387008\r\n\r\nnote: similar issues under ` [volume type: blockfswithoutformat]` and ` [volume type: dir-link] ` were notes in 1.20 release - https://github.com/kubernetes/sig-release/blame/master/releases/release-1.20/meeting-updates/ci-status/rel-full.md#l882\r\n\r\n/sig storage",
            "using enzyme lib write test for react component \r\n\r\nyou should test:\r\n- [ ] component rendering\r\n- [ ] state changes\r\n- [ ] props changes\r\n- [ ] event handlers\r\n- [ ] if...else\r\n- [ ] classes \r\n- [ ] hooks\r\n- [ ] children components\r\n"
        ];
        
        for t in texts {
            let res = clean_text_with_markdown(t);
            println!("{res:?}");
            println!("-------------------------------------------");
        }
    }
}
