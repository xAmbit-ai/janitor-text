use std::io::Cursor;

use html5ever::{parse_document, tendril::TendrilSink, tree_builder::TreeSink};
use markup5ever_rcdom::{Handle, NodeData, RcDom};

pub fn clean_text_with_html(text: &str) -> Vec<String> {
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

        NodeData::Comment { ref contents } => {
            let txt = contents.trim().escape_default().to_string();
            if !txt.is_empty() {
                texts.push(txt);
            }
        }

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

pub fn clean_text(input: &str) -> Vec<String> {
    let mut text = Vec::new();
    let mut last = String::new();
    let mut lastsplchar = ' ';

    let mut escaped = false;

    for c in input.chars() {
        // handelling consecutive punctuations
        if c.is_ascii_punctuation() && c == lastsplchar {
            continue;
        }
        lastsplchar = c;

        if c.is_whitespace() {
            if !last.is_empty() {
                push_cleaned_text(&mut text, last.clone());
                last = String::new();
                if c != ' ' {
                    text.push("[SEP]".to_string());
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
                    push_cleaned_text(&mut text, last.trim().to_owned());
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
                push_cleaned_text(&mut text, last.trim().to_owned());
                last = String::new();
            }

            push_cleaned_text(&mut text, c.to_string());
            continue;
        }

        if (c == ':' || c == '.') && !last.starts_with("http") {
            if !last.is_empty() {
                push_cleaned_text(&mut text, last.trim().to_owned());
                last = String::new();
            }

            push_cleaned_text(&mut text, c.to_string());
            continue;
        }

        last.push(c);
    }

    if !last.is_empty() {
        push_cleaned_text(&mut text, last.clone());
    }

    text
}

fn push_cleaned_text(d: &mut Vec<String>, txt: String) {
    if txt.starts_with("http:/") || txt.starts_with("https:/") {
        d.push("link".to_string());
        return;
    }

    if let Some(p) = d.last() {
        // ignoring if last two words are exactly the same
        if p == &txt {
            return;
        }
    }

    d.push(txt)
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
    // #[test]
    // fn it_works() {
    //     let result = add(2, 2);
    //     assert_eq!(result, 4);
    // }
}
