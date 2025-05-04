#ifndef NEWSBOAT_TTRSSURLREADER_H_
#define NEWSBOAT_TTRSSURLREADER_H_

#include "urlreader.h"

namespace newsboat {

class RemoteApi;

class TtRssUrlReader : public UrlReader {
public:
	TtRssUrlReader(const std::string& url_file, RemoteApi* a);
	~TtRssUrlReader() override;
	std::optional<utils::ReadTextFileError> reload() override;
	std::string get_source() const override;

private:
	std::string file;
	RemoteApi* api;
};

} // namespace newsboat

#endif /* NEWSBOAT_TTRSSURLREADER_H_ */
