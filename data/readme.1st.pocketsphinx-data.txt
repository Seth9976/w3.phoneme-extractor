Language and phoneme model for pocketsphinx has to be downloaded from the pocketsphinx
src repository:

- create a subfolder "en" in the data/pocketsphinx folder

- download the en-us language model (content of the folder) from

	https://github.com/cmusphinx/pocketsphinx/tree/master/model/en-us/en-us/

  and put it directly into the data/pocketsphinx/en folder

- download the en-us phoneme model and en-us language dictionary from

	https://github.com/cmusphinx/pocketsphinx/blob/master/model/en-us/en-us-phone.lm.bin

	https://github.com/cmusphinx/pocketsphinx/blob/master/model/en-us/cmudict-en-us.dict

  and put them directly into the data/pocketsphinx/en folder

- rename:
	- en-us-phone.lm.bin -> en-phone.lm.bin
	- cmudict-en-us.dict -> en-language.dict

- the pocketsphinx/<language-code>-custom.dict and pocketsphinx/<language-code>-phoneme-mappng.cfg
  are additional configuration files for the phoneme extractor and are not part
  of any pocketsphinx distribution/packages
