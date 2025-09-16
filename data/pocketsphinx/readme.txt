# language support

The default download of the phoneme extractor contains only the CMUSphinx
models for "en-us". They are located in the directory "en".

Other languages supported by CMUSphinx *may* work. You have to try for
yourself. From the website:

https://cmusphinx.github.io/wiki/faq/#q-which-languages-are-supported:

"CMUSphinx itself is language-independent, you can recognize any language.
However, it requires an acoustic model and a language model. We provide
prebuilt language models for many languages (English, Chinese, French,
Spanish, German, Russian, etc) in download section."

# NOTE:
The phoneme extractor searches for a data/pocketsphinx/<language-code>
subdirectory named as the lowercased language code provided by the
--language option. this directory must contain:
	- an appropriate CMUSphinx language model and
	- an appropriate <language-code>-phone.lm.bin (phoneme model)
	- an appropriate <language-code>-language.dict (dictionary with words mapping to phoeme sequences)

A language dictionary must be renamed to <language-code>-language.dict
(e.g. cmudict-en-us.dict -> en-language.dict) and copied into the subdirectory.

Good luck.
